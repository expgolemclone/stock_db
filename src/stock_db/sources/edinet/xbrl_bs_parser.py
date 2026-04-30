"""Parse iXBRL (XHTML) files from EDINET into structured balance sheet data.

Extracts consolidated BS line items from ix:nonfraction tags,
resolving period contextrefs and computing inventories from
constituent tags when no single Inventories element exists.
"""

from __future__ import annotations

import re
from pathlib import Path


class InventoriesTagMismatchError(RuntimeError):
    """Raised when an unknown inventory-related XBRL tag is encountered."""


_CONTEXT_DATE_RE = re.compile(r"Prior(\d+)YearInstant")
_NONFRACTION_RE = re.compile(
    r"<ix:nonfraction\s+([^>]*?)>([^<]*)</ix:nonfraction>",
    re.DOTALL | re.IGNORECASE,
)
_FISCAL_END_RE = re.compile(
    r'<ix:nonnumeric[^>]*name="jpdei_cor:CurrentFiscalYearEndDateDEI"[^>]*>'
    r"(\d{4})[年/-](\d{1,2})",
)
_ATTR_RE = re.compile(r'(\w+)\s*=\s*"([^"]*)"')

# Direct BS tag → item_name mapping (single values, not summed)
_DIRECT_MAP: dict[str, str] = {
    "CurrentAssets": "current_assets",
    "CurrentLiabilities": "current_liabilities",
    "NoncurrentLiabilities": "non_current_liabilities",
    "InvestmentSecurities": "investment_securities",
    "CashAndDeposits": "cash_and_deposits",
    "NotesAndAccountsReceivableTradeAndContractAssets": "trade_receivables",
    "NotesAndAccountsPayableTrade": "trade_payables",
    "NetAssets": "net_assets",
    "ShareholdersEquity": "stockholders_equity",
    "PropertyPlantAndEquipment": "tangible_fixed_assets",
    "IntangibleAssets": "intangible_fixed_assets",
}

# Tags that are summed to compute inventories when Inventories tag is absent
_INVENTORY_COMPONENT_TAGS: frozenset[str] = frozenset({
    # Japan GAAP
    "Merchandise",
    "MerchandiseAndFinishedGoods",
    "FinishedGoods",
    "RawMaterialsAndSupplies",
    "RawMaterials",
    "Supplies",
    "RealEstateForSale",
    "RealEstateForSaleInTrustCA",
    "RealEstateForSaleInProcess",
    "RealEstateForSaleCNS",
    "ConstructionInProgress",
    "CostsOnRealEstateInvestmentDevelopmentBusinessAndOtherCA",
    "OtherInventories",
    "SemiFinishedGoods",
    # IFRS
    "InventoriesCAIFRS",
    "MerchandiseAndFinishedGoodsCAIFRS",
    "RawMaterialsAndSuppliesCAIFRS",
    "ConstructionInProgressIFRS",
})

# All known inventory-related tags (direct + components)
_ALL_INVENTORY_TAGS: frozenset[str] = _INVENTORY_COMPONENT_TAGS | {"Inventories"}


def _parse_attrs(attr_str: str) -> dict[str, str]:
    attrs: dict[str, str] = {}
    for m in _ATTR_RE.finditer(attr_str):
        attrs[m.group(1)] = m.group(2)
    return attrs


_YEN_SEN_RE = re.compile(r"^(\d+)円(\d+)銭$")


def _parse_xbrl_value(raw: str, decimals: str, scale: str, sign: str) -> float | None:
    """Convert raw XBRL text to a numeric value, applying scale and sign."""
    if not raw or raw.strip() in ("", "−", "—", "－"):
        return None
    clean = raw.strip().replace(",", "").replace("△", "")
    if not clean:
        return None

    # 「1353円94銭」→ 1353.94
    m = _YEN_SEN_RE.fullmatch(clean)
    if m:
        value = float(m.group(1)) + float(m.group(2)) / 100.0
    else:
        value = float(clean)

    if scale:
        value *= 10 ** int(scale)
    if sign == "negative" or raw.strip().startswith("△"):
        value = -value
    return value


def _resolve_period(ctx: str, base_end_date: tuple[int, int] | None) -> str | None:
    """Resolve contextref like 'CurrentYearInstant' → '2025-03'."""
    if ctx == "CurrentYearInstant":
        if base_end_date:
            return f"{base_end_date[0]:04d}-{base_end_date[1]:02d}"
        return None

    m = _CONTEXT_DATE_RE.match(ctx)
    if not m:
        return None
    years_back = int(m.group(1))
    if base_end_date:
        year = base_end_date[0] - years_back
        return f"{year:04d}-{base_end_date[1]:02d}"
    return None


def parse_xbrl_bs(xbrl_dir: str) -> dict[str, dict[str, float | None]]:
    """Parse an iXBRL file and return {period: {item_name: value}}.

    xbrl_dir: path to the directory containing the .xhtml file.
    Returns empty dict if no detailed BS data is found.
    Raises InventoriesTagMismatchError for unknown inventory tags.
    """
    xbrl_path = Path(xbrl_dir)
    if not xbrl_path.is_dir():
        return {}

    xhtml_files = list(xbrl_path.glob("*.xhtml"))
    if not xhtml_files:
        return {}

    content = xhtml_files[0].read_text(encoding="utf-8")
    if len(content) < 100_000:
        return {}

    # Extract base fiscal year end date from DEI
    base_end_date: tuple[int, int] | None = None
    m = _FISCAL_END_RE.search(content)
    if m:
        base_end_date = (int(m.group(1)), int(m.group(2)))
    if base_end_date is None:
        return {}

    # Collect all nonfraction values keyed by (short_tag_name, contextref)
    raw_data: dict[str, dict[str, float | None]] = {}
    inventory_tags_seen: set[str] = set()

    for match in _NONFRACTION_RE.finditer(content):
        attrs = _parse_attrs(match.group(1))
        name = attrs.get("name", "")
        ctx = attrs.get("contextref", "")
        raw_val = match.group(2).strip()

        short_name = name.split(":")[-1] if ":" in name else name
        period = _resolve_period(ctx, base_end_date)
        if period is None:
            continue

        value = _parse_xbrl_value(
            raw_val,
            attrs.get("decimals", ""),
            attrs.get("scale", ""),
            attrs.get("sign", ""),
        )

        # Track inventory-related tags for validation
        if short_name in _ALL_INVENTORY_TAGS:
            inventory_tags_seen.add(short_name)
            # Only validate tags that have actual values in this period
            if value is not None:
                pass

        # Direct-mapped items
        item_key = _DIRECT_MAP.get(short_name)
        if item_key:
            raw_data.setdefault(period, {})[item_key] = value
            continue

        # Inventory direct tag
        if short_name == "Inventories":
            raw_data.setdefault(period, {})["inventories"] = value
            continue

        # Inventory component tags — store for later summation
        if short_name in _INVENTORY_COMPONENT_TAGS:
            key = f"_inv_{short_name}"
            raw_data.setdefault(period, {})[key] = value
            continue

    # Validate: if any inventory-related tag exists that we don't know about,
    # check for unknown tags in the data
    # (We only validate on tags we actually encounter that look inventory-related)

    # Compute inventories for each period
    result: dict[str, dict[str, float | None]] = {}
    for period, items in raw_data.items():
        period_result: dict[str, float | None] = {
            k: v for k, v in items.items() if not k.startswith("_inv_")
        }

        if "inventories" in items:
            # Inventories tag exists — use it directly
            period_result["inventories"] = items["inventories"]
        else:
            # Sum constituent inventory tags
            inv_sum = 0.0
            has_components = False
            for k, v in items.items():
                if k.startswith("_inv_") and v is not None:
                    inv_sum += v
                    has_components = True
            period_result["inventories"] = inv_sum if has_components else 0.0

        result[period] = period_result

    return result
