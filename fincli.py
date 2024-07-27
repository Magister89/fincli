#!/usr/bin/env python3
"""
Finance CLI for checking stocks/funds info and your portfolio status
"""
from typing import Optional
from requests_cache.backends.filesystem import FileCache
from requests_ratelimiter import MemoryQueueBucket
from pyrate_limiter import Duration, RequestRate, Limiter

import typer
from typing_extensions import Annotated
import rich_functions as rf
import fincli_cache as fcache
import portfolio as prt
import ticker as tk

PORTFOLIO_CACHE = 'fincli_portfolio'
TICKER_CACHE = 'fincli_ticker'
app = typer.Typer()


def create_portfolio_cache():
    """
    Creates Portfolio cache
    """
    cache = FileCache(cache_name=PORTFOLIO_CACHE)
    return fcache.CachedLimiterSession(
        limiter=Limiter(RequestRate(1, Duration.SECOND*3)),
        bucket_class=MemoryQueueBucket,
        backend=cache, expire_after=300)


def create_ticker_cache():
    """
    Creates Ticker cache
    """
    cache = FileCache(cache_name=TICKER_CACHE)
    return fcache.CachedLimiterSession(
        limiter=Limiter(RequestRate(1, Duration.SECOND*3)),
        bucket_class=MemoryQueueBucket,
        backend=cache, expire_after=300)


@app.callback()
def main(ctx: typer.Context):
    """
    Cache callback
    """
    ctx.obj = {}
    if ctx.invoked_subcommand == 'ticker':
        ctx.obj['ticker'] = create_ticker_cache()
    if ctx.invoked_subcommand == 'portfolio':
        ctx.obj['portfolio'] = create_portfolio_cache()


@app.command()
def ticker(ctx: typer.Context,
           tkr: str, info: Annotated[Optional[str], typer.Argument(help="Prints Info")] = None,
           attribute: str = typer.Option("previousClose",
                                         "--attribute", "-a", help="Attribute value")):
    """
    Stocks/Funds info
    """
    tkr_loaded = tk.Ticker(tkr, ctx.obj['ticker'])
    rf.prettier_info(tkr_loaded, info, attribute)


@app.command()
def portfolio(ctx: typer.Context, total: bool = typer.Option(False, "--total", "-t",
                                                             help="Prints total value and yield",
                                                             is_flag=True),
              file: str = typer.Option("portfolio.json", "--file", "-f", help="File Path")):
    """
    Portfolio Status
    """
    session = ctx.obj['portfolio']
    prt_loaded = prt.Portfolio(file, session)
    rf.prettier_portfolio(prt_loaded, total)


if __name__ == "__main__":
    app()
