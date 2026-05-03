"""Parse EDINET iXBRL into inventories-only balance sheet data."""

from __future__ import annotations

import logging
import re
from pathlib import Path

logger = logging.getLogger(__name__)


class InventoriesTagMismatchError(RuntimeError):
    """Raised when an unknown inventory-like XBRL tag is encountered."""


_CONTEXT_DATE_RE = re.compile(r"^Prior(\d+)YearInstant$")
_NONFRACTION_RE = re.compile(
    r"<ix:nonfraction\s+([^>]*?)>([^<]*)</ix:nonfraction>",
    re.DOTALL | re.IGNORECASE,
)
_NONFRACTION_TAG_RE = re.compile(r"<ix:nonfraction\b", re.IGNORECASE)
_FISCAL_END_RE = re.compile(
    r'<ix:nonnumeric[^>]*name="jpdei_cor:CurrentFiscalYearEndDateDEI"[^>]*>'
    r"(\d{4})[年/-](\d{1,2})",
    re.IGNORECASE,
)
_ATTR_RE = re.compile(r'(\w+)\s*=\s*"([^"]*)"')

_INVENTORY_TOTAL_TAGS: frozenset[str] = frozenset({
    "Inventories",
    "InventoriesCA",
    "InventoriesCAIFRS",
    "InventoriesIFRS",
    "InventoriesAssetsIFRS",
})

_INVENTORY_COMPONENT_TAGS: frozenset[str] = frozenset({
    # Japan GAAP
    "BeautyMaterialsCA",
    "EducationalMaterialsCA",
    "EquipmentAndMaterials",
    "FinishedGoods",
    "FinishedGoodsAndSemiFinishedGoodsCA",
    "FinishedGoodsAndWorkInProcess",
    "FinishedGoodsAndWorkInProcessCA",
    "FinishedGoodsCA",
    "FinishedGoodsCAIFRS",
    "FinishedGoodsIncludingSemiFinishedGoods",
    "FoodInventoryOnStoreCAAssets",
    "IngredientsAndProductionSuppliesCA",
    "InventoriesOfJointProjectInvestmentCA",
    "MaintenanceSupplies",
    "MaterialsAndStocksCA",
    "Merchandise",
    "MerchandiseAndFinishedGoods",
    "MerchandiseAndFinishedGoodsSemiFinishedGoods",
    "MerchandiseCA",
    "MerchandiseEtcCA",
    "MerchandiseLentCA",
    "MerchandiseSuppliesCA",
    "OtherInventories",
    "PartlyFinishedGoodsCA",
    "PFIProjectsAndOtherInventoriesCA",
    "ProgramInventories",
    "PurchasedGoodsMaterialsAndSuppliesCA",
    "RawMaterials",
    "RawMaterialsAndSupplies",
    "RawMaterialsAndSuppliesCA",
    "RawMaterialsAndSuppliesCNS",
    "RawMaterialsCA",
    "RawMaterialsCAGAS",
    "RawMaterialsInTransit",
    "RealEstateForSale",
    "RealEstateForSaleAndDevelopmentProjectsInProgressCNS",
    "RealEstateForSaleCNS",
    "RealEstateForSaleInProcess",
    "RealEstateForSaleInProcessCA",
    "RealEstateForSaleInProcessAndOtherCA",
    "RealEstateForSaleInTrustCA",
    "RentalInventoryAssetsCA",
    "SemiFinishedGoods",
    "SemiFinishedGoodsAndWorkInProcessCA",
    "Supplies",
    "SuppliesCA",
    "TeachingMaterials",
    "TemporaryMaterials",
    "TrustBeneficiaryRightOfRealEstateForSaleInventories",
    "UndeliveredMerchandise",
    "WorkInProcess",
    "WorkInProcessAndPartlyFinishedConstructionCA",
    "WorkInProcessCA",
    "WorkInProcessCAAssets",
    "WorkInProcessContentsAssetsCA",
    "WorkInProcessConstructionCA",
    # IFRS
    "ConstructionInProgressCAIFRS",
    "FinishedGoodsCAIFRS",
    "InventoriesOfJointProjectInvestmentCA",
    "MerchandiseAndFinishedGoodsCAIFRS",
    "MerchandiseAssetsIFRS",
    "MerchandiseCAIFRS",
    "OtherInventoriesAssetsIFRS",
    "OtherInventoriesCAIFRS",
    "ProductionSuppliesCAIFRS",
    "ProgramInventories",
    "RawMaterialsAndOthersCAIFRS",
    "RawMaterialsAndSuppliesCAIFRS",
    "RawMaterialsPurchasedComponentsAndSuppliesCAIFRS",
    "RawMaterialsWorkInProgressAndSuppliesCAIFRS",
    "RawMaterialsCAIFRS",
    "RealEstateForSaleAssetsIFRS",
    "RealEstateForSaleCAIFRS",
    "RealEstateForSaleInProcessCAIFRS",
    "SemiFinishedGoodsAndWorkInProcessCAIFRS",
    "SemiFinishedGoodsCAIFRS",
    "SuppliesAndOtherCAIFRS",
    "SuppliesAndRawMaterialsCAIFRS",
    "TelecommunicationsTerminalEquipmentAndMaterialsToBeSoldCAIFRS",
    "WorkInProcessAndRawMaterialsCAIFRS",
    "WorkInProcessAssetsIFRS",
    "WorkInProcessCAIFRS",
})

_IGNORED_INVENTORY_SUBSTRINGS: tuple[str, ...] = (
    "AccountsReceivable",
    "AccumulatedDepreciation",
    "AccumulatedImpairment",
    "AcquisitionCost",
    "Adjustment",
    "AllowanceFor",
    "Amortization",
    "Beginning",
    "ChangeIn",
    "ChangesIn",
    "Compensation",
    "CostOf",
    "DecreaseIncrease",
    "DifferenceBetweenCostOf",
    "Disposal",
    "Ending",
    "ExportPriceAdjustment",
    "GainOn",
    "GrossProfit",
    "IfDifferentFromBsBalance",
    "IncreaseDecrease",
    "LossOn",
    "NCA",
    "NetCOS",
    "NetSales",
    "Notes",
    "OfWhich",
    "PaymentsFor",
    "ProceedsFrom",
    "ProvisionFor",
    "Purchase",
    "Receivable",
    "Recycled",
    "Redemption",
    "ReserveFor",
    "ScheduledToBeSold",
    "Subtotal",
    "ThatAreScheduledToBeSold",
    "TextBlock",
    "ToBeSoldForMoreThan",
    "ToBeSoldMoreThan",
    "TotalBeginning",
    "TransferFrom",
    "TransferTo",
    "Valuation",
    "WriteDown",
)

_INVENTORY_CANDIDATE_KEYWORDS: tuple[str, ...] = (
    "BeautyMaterials",
    "EducationalMaterials",
    "FinishedGoods",
    "FoodInventory",
    "IngredientsAndProductionSupplies",
    "Inventor",
    "MaintenanceSupplies",
    "Materials",
    "Merchandise",
    "OtherInventories",
    "ProgramInventories",
    "RawMaterials",
    "RealEstateForSale",
    "RentalInventory",
    "SemiFinishedGoods",
    "Stocks",
    "Supplies",
    "TeachingMaterials",
    "TemporaryMaterials",
    "UndeliveredMerchandise",
    "WorkInProcess",
)

_YEN_SEN_RE = re.compile(r"^(\d+)円(\d+)銭$")


def _parse_attrs(attr_str: str) -> dict[str, str]:
    attrs: dict[str, str] = {}
    for match in _ATTR_RE.finditer(attr_str):
        attrs[match.group(1)] = match.group(2)
    return attrs


def _parse_xbrl_value(raw: str, decimals: str, scale: str, sign: str) -> float | None:
    """Convert raw XBRL text to a numeric value, applying scale and sign."""
    if not raw or raw.strip() in ("", "−", "—", "－"):
        return None
    clean = raw.strip().replace(",", "").replace("△", "")
    if not clean:
        return None

    yen_sen = _YEN_SEN_RE.fullmatch(clean)
    if yen_sen:
        value = float(yen_sen.group(1)) + float(yen_sen.group(2)) / 100.0
    else:
        value = float(clean)

    if scale:
        value *= 10 ** int(scale)
    if sign == "negative" or raw.strip().startswith("△"):
        value = -value
    return value


def _resolve_period(ctx: str, base_end_date: tuple[int, int] | None) -> str | None:
    """Resolve exact consolidated instant contexts into YYYY-MM periods."""
    if base_end_date is None:
        return None
    if ctx == "CurrentYearInstant":
        return f"{base_end_date[0]:04d}-{base_end_date[1]:02d}"

    match = _CONTEXT_DATE_RE.match(ctx)
    if match is None:
        return None
    year = base_end_date[0] - int(match.group(1))
    return f"{year:04d}-{base_end_date[1]:02d}"


def is_valid_xbrl_text(content: str) -> bool:
    """Return True when the payload looks like a parseable EDINET iXBRL body."""
    return _NONFRACTION_TAG_RE.search(content) is not None and _FISCAL_END_RE.search(content) is not None


def is_valid_xbrl_path(path: str | Path | None) -> bool:
    """Return True when the saved XBRL file exists and passes minimal validation."""
    if path is None:
        return False
    xbrl_path = Path(path)
    if not xbrl_path.is_file():
        return False
    try:
        return is_valid_xbrl_text(xbrl_path.read_text(encoding="utf-8"))
    except OSError:
        logger.warning("Failed to read XBRL file %s", xbrl_path)
        return False


def _is_inventory_like(short_name: str) -> bool:
    return any(keyword in short_name for keyword in _INVENTORY_CANDIDATE_KEYWORDS)


def _is_ignored_inventory_candidate(short_name: str) -> bool:
    if short_name.startswith("ConstructionInProgress"):
        return short_name != "ConstructionInProgressCAIFRS"
    return any(fragment in short_name for fragment in _IGNORED_INVENTORY_SUBSTRINGS)


def _store_fact(
    bucket: dict[str, dict[str, float | None]],
    period: str,
    short_name: str,
    value: float | None,
) -> None:
    by_period = bucket.setdefault(period, {})
    if short_name not in by_period:
        by_period[short_name] = value
        return

    existing = by_period[short_name]
    if existing is None:
        by_period[short_name] = value
        return
    if value is None or existing == value:
        return
    raise InventoriesTagMismatchError(
        f"Conflicting values for inventory tag {short_name} in {period}: {existing} vs {value}"
    )


def parse_xbrl_bs(xbrl_dir: str) -> dict[str, dict[str, float | None]]:
    """Parse an iXBRL directory and return {period: {'inventories': value}}."""
    xbrl_path = Path(xbrl_dir)
    if not xbrl_path.is_dir():
        return {}

    xhtml_files = list(xbrl_path.glob("*.xhtml"))
    if not xhtml_files:
        return {}

    target = max(xhtml_files, key=lambda path: path.stat().st_size)
    content = target.read_text(encoding="utf-8")
    if not is_valid_xbrl_text(content):
        return {}

    fiscal_end = _FISCAL_END_RE.search(content)
    if fiscal_end is None:
        return {}
    base_end_date = (int(fiscal_end.group(1)), int(fiscal_end.group(2)))

    direct_totals: dict[str, dict[str, float | None]] = {}
    component_values: dict[str, dict[str, float | None]] = {}
    periods_seen: set[str] = set()
    unknown_tags: set[str] = set()

    for match in _NONFRACTION_RE.finditer(content):
        attrs = _parse_attrs(match.group(1))
        short_name = attrs.get("name", "").split(":")[-1]
        period = _resolve_period(attrs.get("contextref", ""), base_end_date)
        if period is None:
            continue

        periods_seen.add(period)
        value = _parse_xbrl_value(
            match.group(2).strip(),
            attrs.get("decimals", ""),
            attrs.get("scale", ""),
            attrs.get("sign", ""),
        )

        if short_name in _INVENTORY_TOTAL_TAGS:
            _store_fact(direct_totals, period, short_name, value)
            continue

        if short_name in _INVENTORY_COMPONENT_TAGS:
            _store_fact(component_values, period, short_name, value)
            continue

        if value is None or not _is_inventory_like(short_name):
            continue
        if _is_ignored_inventory_candidate(short_name):
            continue
        unknown_tags.add(short_name)

    if unknown_tags:
        unknown_list = ", ".join(sorted(unknown_tags))
        raise InventoriesTagMismatchError(f"Unknown inventory-like XBRL tags: {unknown_list}")

    if not periods_seen:
        return {}

    result: dict[str, dict[str, float | None]] = {}
    for period in sorted(periods_seen, reverse=True):
        total_candidates = [
            value
            for value in direct_totals.get(period, {}).values()
            if value is not None
        ]
        if total_candidates:
            unique_totals = sorted(set(total_candidates))
            if len(unique_totals) > 1:
                raise InventoriesTagMismatchError(
                    f"Conflicting direct inventory totals in {period}: {unique_totals}"
                )
            inventories = unique_totals[0]
        else:
            inventories = sum(
                value
                for value in component_values.get(period, {}).values()
                if value is not None
            )
        result[period] = {"inventories": inventories}

    return result
