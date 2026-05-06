"""Parse EDINET XBRL artifacts into inventories-only balance sheet data."""

from __future__ import annotations

import logging
import re
import xml.etree.ElementTree as ET
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote, urlparse

logger = logging.getLogger(__name__)


class InventoriesTagMismatchError(RuntimeError):
    """Raised when the inventories total cannot be determined safely."""


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
_YEN_SEN_RE = re.compile(r"^(\d+)円(\d+)銭$")

_XML_NS = "http://www.w3.org/XML/1998/namespace"
_XHTML_NS = "http://www.w3.org/1999/xhtml"
_IX_NS = "http://www.xbrl.org/2008/inlineXBRL"
_XBRLI_NS = "http://www.xbrl.org/2003/instance"
_XBRLDI_NS = "http://xbrl.org/2006/xbrldi"
_XLINK_NS = "http://www.w3.org/1999/xlink"
_LINK_NS = "http://www.xbrl.org/2003/linkbase"
_XS_NS = "http://www.w3.org/2001/XMLSchema"

_IGNORED_FACT_NAMESPACES: frozenset[str] = frozenset({
    _XHTML_NS,
    _IX_NS,
    _XBRLI_NS,
    _XBRLDI_NS,
    _XLINK_NS,
    _LINK_NS,
    _XS_NS,
    _XML_NS,
})

_FACT_FILE_PATTERNS: tuple[str, ...] = ("*.xhtml", "*.html", "*.htm", "*.xbrl")

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
    # Japan GAAP - 不動産・建設業
    "CostsOnRealEstateBusiness",
    "CostsOnUncompletedConstructionContractsCNS",
    "CostsOnUncompletedConstructionContractsAndOtherCNS",
    "CostsOnUncompletedServices",
    "DevelopmentProjectsInProgress",
    "GoodsInTransit",
    "LandAndBuildingsForSaleInLots",
    "LandForSaleInLots",
    "MerchandizeAndFinishedGoods",
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
    "ProductsStocks",
    "ProvisionFor",
    "Purchase",
    "Receivable",
    "Recycled",
    "Redemption",
    "ReserveFor",
    "StocksOf",
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

ConceptKey = tuple[str, str]


@dataclass(frozen=True, slots=True)
class _ContextInfo:
    period: str | None
    instant: str | None
    is_instant: bool
    has_dimensions: bool
    is_non_consolidated: bool


@dataclass(frozen=True, slots=True)
class _ParsedDocument:
    path: Path
    root: ET.Element
    nsmap: dict[str, str]


@dataclass(frozen=True, slots=True)
class _CalculationEdge:
    child: ConceptKey
    weight: float


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
    """Return True when the saved XBRL artifact exists and passes minimal validation."""
    if path is None:
        return False
    xbrl_path = Path(path)
    if xbrl_path.is_file():
        try:
            return is_valid_xbrl_text(xbrl_path.read_text(encoding="utf-8"))
        except OSError:
            logger.warning("Failed to read XBRL file %s", xbrl_path)
            return False
    if not xbrl_path.is_dir():
        return False

    zip_path = xbrl_path.parent / f"{xbrl_path.name}.zip"
    if zip_path.is_file():
        for candidate in _iter_fact_document_paths(xbrl_path):
            if candidate.suffix.lower() == ".xbrl":
                return True
            try:
                if is_valid_xbrl_text(candidate.read_text(encoding="utf-8")):
                    return True
            except OSError:
                logger.warning("Failed to read XBRL artifact file %s", candidate)
        return False

    if _is_legacy_xbrl_dir(xbrl_path):
        target = _legacy_target_file(xbrl_path)
        return target is not None and is_valid_xbrl_path(target)
    return False


def _is_inventory_like(short_name: str) -> bool:
    return any(keyword in short_name for keyword in _INVENTORY_CANDIDATE_KEYWORDS)


def _is_ignored_inventory_candidate(short_name: str) -> bool:
    if short_name.startswith("ConstructionInProgress"):
        return short_name != "ConstructionInProgressCAIFRS"
    return any(fragment in short_name for fragment in _IGNORED_INVENTORY_SUBSTRINGS)


def _local_name(tag: str) -> str:
    return tag.split("}", 1)[1] if tag.startswith("{") else tag


def _namespace_uri(tag: str) -> str:
    return tag[1:].split("}", 1)[0] if tag.startswith("{") else ""


def _iter_fact_document_paths(root: Path) -> list[Path]:
    paths: list[Path] = []
    seen: set[Path] = set()
    for pattern in _FACT_FILE_PATTERNS:
        for path in sorted(root.rglob(pattern)):
            if path.is_file() and path not in seen:
                seen.add(path)
                paths.append(path)
    return paths


def _collect_nsmap(path: Path) -> dict[str, str]:
    nsmap: dict[str, str] = {}
    try:
        for _, node in ET.iterparse(path, events=("start-ns",)):
            prefix, uri = node
            nsmap[prefix or ""] = uri
    except ET.ParseError:
        return {}
    return nsmap


def _parse_xml_document(path: Path) -> _ParsedDocument | None:
    try:
        root = ET.parse(path).getroot()
    except (ET.ParseError, OSError) as exc:
        logger.warning("Failed to parse XML document %s: %s", path, exc)
        return None
    return _ParsedDocument(path=path, root=root, nsmap=_collect_nsmap(path))


def _resolve_qname(name: str, nsmap: dict[str, str], fallback_uri: str = "") -> ConceptKey | None:
    if not name:
        return None
    if name.startswith("{") and "}" in name:
        uri = _namespace_uri(name)
        return (uri, _local_name(name))
    if ":" in name:
        prefix, local = name.split(":", 1)
        uri = nsmap.get(prefix)
        if uri is None:
            return None
        return (uri, local)
    if fallback_uri:
        return (fallback_uri, name)
    uri = nsmap.get("")
    return (uri, name) if uri else None


def _context_period_from_instant(instant_text: str | None) -> str | None:
    if not instant_text or len(instant_text) < 7:
        return None
    return instant_text[:7]


def _parse_contexts(root: ET.Element) -> dict[str, _ContextInfo]:
    contexts: dict[str, _ContextInfo] = {}
    for elem in root.iter():
        if _namespace_uri(elem.tag) != _XBRLI_NS or _local_name(elem.tag) != "context":
            continue
        context_id = elem.attrib.get("id")
        if not context_id:
            continue

        instant_text: str | None = None
        is_instant = False
        has_dimensions = False
        is_non_consolidated = False

        for child in elem.iter():
            namespace = _namespace_uri(child.tag)
            local_name = _local_name(child.tag)
            if namespace == _XBRLI_NS and local_name == "instant":
                instant_text = (child.text or "").strip()
                is_instant = bool(instant_text)
                continue
            if namespace != _XBRLDI_NS:
                continue
            if local_name == "explicitMember":
                has_dimensions = True
                dimension_name = child.attrib.get("dimension", "").split(":")[-1]
                member_name = (child.text or "").strip().split(":")[-1]
                if (
                    member_name == "NonConsolidatedMember"
                    or dimension_name == "ConsolidatedOrNonConsolidatedAxis"
                    and member_name == "NonConsolidatedMember"
                ):
                    is_non_consolidated = True
            elif local_name == "typedMember":
                has_dimensions = True

        contexts[context_id] = _ContextInfo(
            period=_context_period_from_instant(instant_text),
            instant=instant_text,
            is_instant=is_instant,
            has_dimensions=has_dimensions,
            is_non_consolidated=is_non_consolidated,
        )
    return contexts


def _parse_units(root: ET.Element) -> dict[str, bool]:
    units: dict[str, bool] = {}
    for elem in root.iter():
        if _namespace_uri(elem.tag) != _XBRLI_NS or _local_name(elem.tag) != "unit":
            continue
        unit_id = elem.attrib.get("id")
        if not unit_id:
            continue
        is_jpy = False
        for child in elem.iter():
            if _namespace_uri(child.tag) != _XBRLI_NS or _local_name(child.tag) != "measure":
                continue
            measure_text = (child.text or "").strip()
            if measure_text.split(":")[-1] == "JPY":
                is_jpy = True
                break
        units[unit_id] = is_jpy
    return units


def _is_nil(elem: ET.Element) -> bool:
    for key, value in elem.attrib.items():
        if _local_name(key) == "nil" and value.lower() == "true":
            return True
    return False


def _should_use_context(context: _ContextInfo | None) -> bool:
    return bool(
        context is not None
        and context.is_instant
        and context.period is not None
        and not context.is_non_consolidated
        and not context.has_dimensions
    )


def _store_concept_value(
    bucket: dict[str, dict[ConceptKey, float | None]],
    period: str,
    concept: ConceptKey,
    value: float | None,
) -> None:
    by_period = bucket.setdefault(period, {})
    if concept not in by_period:
        by_period[concept] = value
        return

    existing = by_period[concept]
    if existing is None:
        by_period[concept] = value
        return
    if value is None or existing == value:
        return
    raise InventoriesTagMismatchError(
        f"Conflicting values for inventory concept {concept[1]} in {period}: {existing} vs {value}"
    )


def _extract_inline_facts(
    document: _ParsedDocument,
    contexts: dict[str, _ContextInfo],
    units: dict[str, bool],
    facts: dict[str, dict[ConceptKey, float | None]],
    periods_seen: set[str],
) -> None:
    for elem in document.root.iter():
        if _namespace_uri(elem.tag) != _IX_NS or _local_name(elem.tag) != "nonfraction":
            continue
        context_id = elem.attrib.get("contextRef") or elem.attrib.get("contextref")
        context = contexts.get(context_id or "")
        if not _should_use_context(context):
            continue
        unit_id = elem.attrib.get("unitRef") or elem.attrib.get("unitref")
        if unit_id is None or not units.get(unit_id, False):
            continue
        concept = _resolve_qname(elem.attrib.get("name", ""), document.nsmap)
        if concept is None:
            continue
        periods_seen.add(context.period)
        raw_value = "" if _is_nil(elem) else "".join(elem.itertext()).strip()
        value = _parse_xbrl_value(
            raw_value,
            elem.attrib.get("decimals", ""),
            elem.attrib.get("scale", ""),
            elem.attrib.get("sign", ""),
        )
        _store_concept_value(facts, context.period, concept, value)


def _extract_instance_facts(
    document: _ParsedDocument,
    contexts: dict[str, _ContextInfo],
    units: dict[str, bool],
    facts: dict[str, dict[ConceptKey, float | None]],
    periods_seen: set[str],
) -> None:
    for elem in document.root.iter():
        namespace = _namespace_uri(elem.tag)
        if namespace in _IGNORED_FACT_NAMESPACES:
            continue
        context_id = elem.attrib.get("contextRef")
        if context_id is None:
            continue
        context = contexts.get(context_id)
        if not _should_use_context(context):
            continue
        unit_id = elem.attrib.get("unitRef")
        if unit_id is None or not units.get(unit_id, False):
            continue
        concept = _resolve_qname(elem.tag, document.nsmap)
        if concept is None:
            continue
        periods_seen.add(context.period)
        raw_value = "" if _is_nil(elem) else "".join(elem.itertext()).strip()
        value = _parse_xbrl_value(
            raw_value,
            elem.attrib.get("decimals", ""),
            elem.attrib.get("scale", ""),
            elem.attrib.get("sign", ""),
        )
        _store_concept_value(facts, context.period, concept, value)


def _collect_documents(artifact_root: Path) -> list[_ParsedDocument]:
    documents: list[_ParsedDocument] = []
    for path in _iter_fact_document_paths(artifact_root):
        parsed = _parse_xml_document(path)
        if parsed is not None:
            documents.append(parsed)
    return documents


def _build_concept_lookup(artifact_root: Path) -> tuple[dict[tuple[Path, str], ConceptKey], dict[tuple[str, str], ConceptKey]]:
    by_path: dict[tuple[Path, str], ConceptKey] = {}
    by_basename: dict[tuple[str, str], ConceptKey] = {}
    for xsd_path in sorted(artifact_root.rglob("*.xsd")):
        parsed = _parse_xml_document(xsd_path)
        if parsed is None:
            continue
        target_namespace = parsed.root.attrib.get("targetNamespace", "")
        if not target_namespace:
            continue
        for elem in parsed.root.iter():
            if _namespace_uri(elem.tag) != _XS_NS or _local_name(elem.tag) != "element":
                continue
            name = elem.attrib.get("name")
            element_id = elem.attrib.get("id")
            if not name:
                continue
            concept = (target_namespace, name)
            if element_id:
                key = (xsd_path.resolve(), element_id)
                by_path[key] = concept
                by_basename[(xsd_path.name, element_id)] = concept
    return by_path, by_basename


def _resolve_locator_concept(
    href: str,
    base_path: Path,
    by_path: dict[tuple[Path, str], ConceptKey],
    by_basename: dict[tuple[str, str], ConceptKey],
) -> ConceptKey | None:
    if "#" not in href:
        return None
    href_path, fragment = href.rsplit("#", 1)
    if not fragment:
        return None

    if href_path:
        parsed = urlparse(href_path)
        if parsed.scheme:
            basename = Path(unquote(parsed.path)).name
            return by_basename.get((basename, fragment))
        resolved = (base_path.parent / unquote(href_path)).resolve()
        concept = by_path.get((resolved, fragment))
        if concept is not None:
            return concept
        return by_basename.get((resolved.name, fragment))

    return by_path.get((base_path.resolve(), fragment))


def _build_calculation_graphs(
    artifact_root: Path,
    by_path: dict[tuple[Path, str], ConceptKey],
    by_basename: dict[tuple[str, str], ConceptKey],
) -> dict[str, dict[ConceptKey, list[_CalculationEdge]]]:
    graphs: dict[str, dict[ConceptKey, list[_CalculationEdge]]] = defaultdict(lambda: defaultdict(list))
    for linkbase_path in sorted(artifact_root.rglob("*_cal.xml")):
        parsed = _parse_xml_document(linkbase_path)
        if parsed is None:
            continue
        for link in parsed.root.iter():
            if _namespace_uri(link.tag) != _LINK_NS or _local_name(link.tag) != "calculationLink":
                continue
            role = link.attrib.get(f"{{{_XLINK_NS}}}role", "")
            locators: dict[str, ConceptKey] = {}
            for child in link:
                if _namespace_uri(child.tag) != _LINK_NS or _local_name(child.tag) != "loc":
                    continue
                label = child.attrib.get(f"{{{_XLINK_NS}}}label")
                href = child.attrib.get(f"{{{_XLINK_NS}}}href")
                if not label or not href:
                    continue
                concept = _resolve_locator_concept(href, linkbase_path, by_path, by_basename)
                if concept is not None:
                    locators[label] = concept

            for child in link:
                if _namespace_uri(child.tag) != _LINK_NS or _local_name(child.tag) != "calculationArc":
                    continue
                from_label = child.attrib.get(f"{{{_XLINK_NS}}}from")
                to_label = child.attrib.get(f"{{{_XLINK_NS}}}to")
                if from_label not in locators or to_label not in locators:
                    continue
                try:
                    weight = float(child.attrib.get("weight", "1"))
                except ValueError:
                    continue
                graphs[role][locators[from_label]].append(_CalculationEdge(locators[to_label], weight))
    return graphs


def _build_presentation_graphs(
    artifact_root: Path,
    by_path: dict[tuple[Path, str], ConceptKey],
    by_basename: dict[tuple[str, str], ConceptKey],
) -> dict[str, dict[ConceptKey, set[ConceptKey]]]:
    graphs: dict[str, dict[ConceptKey, set[ConceptKey]]] = defaultdict(lambda: defaultdict(set))
    for linkbase_path in sorted(artifact_root.rglob("*_pre.xml")):
        parsed = _parse_xml_document(linkbase_path)
        if parsed is None:
            continue
        for link in parsed.root.iter():
            if _namespace_uri(link.tag) != _LINK_NS or _local_name(link.tag) != "presentationLink":
                continue
            role = link.attrib.get(f"{{{_XLINK_NS}}}role", "")
            locators: dict[str, ConceptKey] = {}
            for child in link:
                if _namespace_uri(child.tag) != _LINK_NS or _local_name(child.tag) != "loc":
                    continue
                label = child.attrib.get(f"{{{_XLINK_NS}}}label")
                href = child.attrib.get(f"{{{_XLINK_NS}}}href")
                if not label or not href:
                    continue
                concept = _resolve_locator_concept(href, linkbase_path, by_path, by_basename)
                if concept is not None:
                    locators[label] = concept

            for child in link:
                if _namespace_uri(child.tag) != _LINK_NS or _local_name(child.tag) != "presentationArc":
                    continue
                from_label = child.attrib.get(f"{{{_XLINK_NS}}}from")
                to_label = child.attrib.get(f"{{{_XLINK_NS}}}to")
                if from_label not in locators or to_label not in locators:
                    continue
                graphs[role][locators[from_label]].add(locators[to_label])
    return graphs


def _matching_concepts(period_facts: dict[ConceptKey, float | None], names: frozenset[str]) -> list[float]:
    values: list[float] = []
    for concept, value in period_facts.items():
        if concept[1] in names and value is not None:
            values.append(value)
    return values


def _unique_candidate(label: str, period: str, values: list[float]) -> float | None:
    if not values:
        return None
    unique_values = sorted(set(values))
    if len(unique_values) > 1:
        raise InventoriesTagMismatchError(f"Conflicting {label} totals in {period}: {unique_values}")
    return unique_values[0]


def _all_graph_concepts_calculation(graph: dict[ConceptKey, list[_CalculationEdge]]) -> set[ConceptKey]:
    concepts = set(graph)
    for edges in graph.values():
        for edge in edges:
            concepts.add(edge.child)
    return concepts


def _all_graph_concepts_presentation(graph: dict[ConceptKey, set[ConceptKey]]) -> set[ConceptKey]:
    concepts = set(graph)
    for children in graph.values():
        concepts.update(children)
    return concepts


def _evaluate_calculation(
    graph: dict[ConceptKey, list[_CalculationEdge]],
    concept: ConceptKey,
    facts: dict[ConceptKey, float | None],
    memo: dict[ConceptKey, float | None],
    visiting: set[ConceptKey],
) -> float | None:
    if concept in memo:
        return memo[concept]
    if concept in visiting:
        return None
    visiting.add(concept)
    child_values: list[float] = []
    for edge in graph.get(concept, []):
        child_value = _evaluate_calculation(graph, edge.child, facts, memo, visiting)
        if child_value is not None:
            child_values.append(edge.weight * child_value)
    visiting.remove(concept)
    result = sum(child_values) if child_values else facts.get(concept)
    memo[concept] = result
    return result


def _reachable_descendants(
    graph: dict[ConceptKey, set[ConceptKey]],
    concept: ConceptKey,
    memo: dict[ConceptKey, set[ConceptKey]],
) -> set[ConceptKey]:
    if concept in memo:
        return memo[concept]
    descendants: set[ConceptKey] = set()
    for child in graph.get(concept, set()):
        descendants.add(child)
        descendants.update(_reachable_descendants(graph, child, memo))
    memo[concept] = descendants
    return descendants


def _presentation_sum(
    graph: dict[ConceptKey, set[ConceptKey]],
    root: ConceptKey,
    facts: dict[ConceptKey, float | None],
) -> float | None:
    descendants_cache: dict[ConceptKey, set[ConceptKey]] = {}
    closure = {root}
    closure.update(_reachable_descendants(graph, root, descendants_cache))
    factful = {concept for concept in closure if facts.get(concept) is not None}
    if not factful:
        return None

    non_deepest: set[ConceptKey] = set()
    for concept in factful:
        for descendant in _reachable_descendants(graph, concept, descendants_cache):
            if descendant in factful:
                non_deepest.add(concept)
                break

    deepest = factful - non_deepest
    return sum(facts[concept] for concept in deepest if facts[concept] is not None)


def _calculation_candidates(
    facts: dict[ConceptKey, float | None],
    graphs: dict[str, dict[ConceptKey, list[_CalculationEdge]]],
) -> list[float]:
    candidates: list[float] = []
    for graph in graphs.values():
        roots = [concept for concept in _all_graph_concepts_calculation(graph) if concept[1] in _INVENTORY_TOTAL_TAGS]
        memo: dict[ConceptKey, float | None] = {}
        for root in roots:
            value = _evaluate_calculation(graph, root, facts, memo, set())
            if value is not None:
                candidates.append(value)
    return candidates


def _presentation_candidates(
    facts: dict[ConceptKey, float | None],
    graphs: dict[str, dict[ConceptKey, set[ConceptKey]]],
) -> list[float]:
    candidates: list[float] = []
    for graph in graphs.values():
        roots = [concept for concept in _all_graph_concepts_presentation(graph) if concept[1] in _INVENTORY_TOTAL_TAGS]
        for root in roots:
            value = _presentation_sum(graph, root, facts)
            if value is not None:
                candidates.append(value)
    return candidates


def _component_sum(facts: dict[ConceptKey, float | None]) -> float:
    return sum(
        value
        for concept, value in facts.items()
        if concept[1] in _INVENTORY_COMPONENT_TAGS and value is not None
    )


def _unknown_inventory_like_tags(facts: dict[ConceptKey, float | None]) -> set[str]:
    unknown_tags: set[str] = set()
    for concept, value in facts.items():
        short_name = concept[1]
        if value is None or not _is_inventory_like(short_name):
            continue
        if _is_ignored_inventory_candidate(short_name):
            continue
        if short_name in _INVENTORY_TOTAL_TAGS or short_name in _INVENTORY_COMPONENT_TAGS:
            continue
        unknown_tags.add(short_name)
    return unknown_tags


def _parse_artifact_dir(artifact_root: Path) -> dict[str, dict[str, float | None]]:
    documents = _collect_documents(artifact_root)
    if not documents:
        return {}

    contexts: dict[str, _ContextInfo] = {}
    units: dict[str, bool] = {}
    for document in documents:
        contexts.update(_parse_contexts(document.root))
        units.update(_parse_units(document.root))

    facts: dict[str, dict[ConceptKey, float | None]] = {}
    periods_seen: set[str] = set()
    for document in documents:
        _extract_inline_facts(document, contexts, units, facts, periods_seen)
        _extract_instance_facts(document, contexts, units, facts, periods_seen)

    if not periods_seen:
        return {}

    by_path, by_basename = _build_concept_lookup(artifact_root)
    calculation_graphs = _build_calculation_graphs(artifact_root, by_path, by_basename)
    presentation_graphs = _build_presentation_graphs(artifact_root, by_path, by_basename)

    result: dict[str, dict[str, float | None]] = {}
    for period in sorted(periods_seen, reverse=True):
        period_facts = facts.get(period, {})
        direct_total = _unique_candidate("direct inventory", period, _matching_concepts(period_facts, _INVENTORY_TOTAL_TAGS))
        if direct_total is not None:
            result[period] = {"inventories": direct_total}
            continue

        calculation_total = _unique_candidate("calculation inventory", period, _calculation_candidates(period_facts, calculation_graphs))
        if calculation_total is not None:
            result[period] = {"inventories": calculation_total}
            continue

        presentation_total = _unique_candidate("presentation inventory", period, _presentation_candidates(period_facts, presentation_graphs))
        if presentation_total is not None:
            result[period] = {"inventories": presentation_total}
            continue

        component_total = _component_sum(period_facts)
        if component_total:
            result[period] = {"inventories": component_total}
            continue

        unknown_tags = _unknown_inventory_like_tags(period_facts)
        if unknown_tags:
            unknown_list = ", ".join(sorted(unknown_tags))
            raise InventoriesTagMismatchError(f"Unknown inventory-like XBRL tags: {unknown_list}")
        result[period] = {"inventories": 0.0}

    return result


def _legacy_target_file(xbrl_dir: Path) -> Path | None:
    xhtml_files = list(xbrl_dir.glob("*.xhtml"))
    if not xhtml_files:
        return None
    return max(xhtml_files, key=lambda path: path.stat().st_size)


def _is_legacy_xbrl_dir(path: Path) -> bool:
    return next(path.glob("*.xhtml"), None) is not None and next(path.rglob("*.xsd"), None) is None


def _store_legacy_fact(
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


def _parse_legacy_content(content: str) -> dict[str, dict[str, float | None]]:
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
            _store_legacy_fact(direct_totals, period, short_name, value)
            continue

        if short_name in _INVENTORY_COMPONENT_TAGS:
            _store_legacy_fact(component_values, period, short_name, value)
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


def _parse_legacy_file(path: Path) -> dict[str, dict[str, float | None]]:
    try:
        content = path.read_text(encoding="utf-8")
    except OSError:
        logger.warning("Failed to read XBRL file %s", path)
        return {}
    return _parse_legacy_content(content)


def parse_xbrl_bs(xbrl_path: str) -> dict[str, dict[str, float | None]]:
    """Parse an EDINET XBRL artifact and return {period: {'inventories': value}}."""
    artifact_path = Path(xbrl_path)
    if artifact_path.is_file():
        return _parse_legacy_file(artifact_path)
    if not artifact_path.is_dir():
        return {}
    if _is_legacy_xbrl_dir(artifact_path):
        target = _legacy_target_file(artifact_path)
        return _parse_legacy_file(target) if target is not None else {}
    return _parse_artifact_dir(artifact_path)
