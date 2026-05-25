from __future__ import annotations

from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[1]
SYSTEMD_USER_DIR = PROJECT_ROOT / "services" / "systemd" / "user"


def _environment_path(unit_name: str) -> list[str]:
    unit_path = SYSTEMD_USER_DIR / unit_name
    for line in unit_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("Environment=PATH="):
            return line.removeprefix("Environment=PATH=").split(":")
    raise AssertionError(f"{unit_name} has no Environment=PATH entry")


def test_user_services_include_nixos_system_tools_path() -> None:
    for unit_name in (
        "stock-db-price-refresh.service",
        "stock-db-downstream-refresh.service",
    ):
        path_entries = _environment_path(unit_name)

        assert "/run/current-system/sw/bin" in path_entries
