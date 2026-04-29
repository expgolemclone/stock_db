from __future__ import annotations


class StooqError(RuntimeError):
    pass


class StooqCaptchaError(StooqError):
    pass


class StooqDownloadError(StooqError):
    pass


class StooqParseError(StooqError):
    pass
