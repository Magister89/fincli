from requests_cache import CacheMixin
from requests_ratelimiter import LimiterMixin
from requests import Session

class CachedLimiterSession(CacheMixin, LimiterMixin, Session):
    pass