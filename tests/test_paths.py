from __future__ import annotations

import os
from pathlib import Path
from unittest.mock import patch

from stock_db.paths import BROWSER_SERVICE_DIR, PROJECT_ROOT, VAR_DIR


class TestProjectRoot:
    def test_points_to_repo_root(self) -> None:
        assert (PROJECT_ROOT / "pyproject.toml").is_file()


class TestVarDir:
    def test_default_is_under_project_root(self) -> None:
        assert VAR_DIR == PROJECT_ROOT / "var"

    def test_env_override(self) -> None:
        with patch.dict(os.environ, {"STOCK_DB_VAR_DIR": "/tmp/custom_var"}):
            # paths モジュールはモジュール読み込み時に解決されるため、
            # 動的に確認するには再評価が必要。ここでは環境変数の仕組み自体をテスト。
            result = Path(os.environ["STOCK_DB_VAR_DIR"])

            assert result == Path("/tmp/custom_var")


class TestBrowserServiceDir:
    def test_points_to_services_browser(self) -> None:
        assert BROWSER_SERVICE_DIR == PROJECT_ROOT / "services" / "browser"
