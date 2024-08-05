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

PORTFOLIO_CACHE = "/tmp/fincli_portfolio"
TICKER_CACHE = "/tmp/fincli_ticker"
PORTFOLIO_FILE_NAME = "portfolio.json"
app = typer.Typer()


def create_portfolio_cache():
    """
    Creates Portfolio cache
    """
    cache = FileCache(cache_name=PORTFOLIO_CACHE)
    return fcache.CachedLimiterSession(
        limiter=Limiter(RequestRate(1, Duration.SECOND*3)),
        bucket_class=MemoryQueueBucket,
        backend=cache, expire_after=419)


def create_ticker_cache():
    """
    Creates Ticker cache
    """
    cache = FileCache(cache_name=TICKER_CACHE)
    return fcache.CachedLimiterSession(
        limiter=Limiter(RequestRate(1, Duration.SECOND*3)),
        bucket_class=MemoryQueueBucket,
        backend=cache, expire_after=300)


def validate_option(ctx: typer.Context, arg: str, opt: str):
    """
    Generic option validation with specified argument
    """
    if ctx.params.get(arg) is not None and ctx.params.get(opt) is not None:
        raise typer.BadParameter(
            f"--{opt} option in not valid with argument {arg}")


@app.callback()
def main(ctx: typer.Context):
    """
    Cache callback
    """
    ctx.obj = {}
    if ctx.invoked_subcommand == "ticker":
        ctx.obj["ticker"] = create_ticker_cache()
    if ctx.invoked_subcommand == "portfolio":
        ctx.obj["portfolio"] = create_portfolio_cache()


@app.command()
def ticker(ctx: typer.Context, tkr: str,
           info: Annotated[Optional[str], typer.Argument(
               help="Prints Info")] = None,
           attribute: Annotated[Optional[str], typer.Option("--attribute", "-a",
                                                            help="Attribute value")] = None):
    """
    Stocks/Funds info
    """
    session = ctx.obj["ticker"]
    validate_option(ctx, "info", "attribute")
    tkr_loaded = tk.Ticker(tkr, session)
    rf.prettier_info(tkr_loaded, info, attribute)


@app.command()
def portfolio(ctx: typer.Context, total: Annotated[bool,
                                                   typer.Option("--total", "-t",
                                                                help="Prints total value and yield",
                                                                is_flag=True)] = False,
              file: Annotated[str, typer.Option("--file", "-f",
                                                          help="File Path")] = PORTFOLIO_FILE_NAME,
              cache: Annotated[bool, typer.Option("--cache", help="Cache renewal", is_flag=True)] = False):
    """
    Portfolio Status
    """
    session = ctx.obj["portfolio"]
    prt_loaded = prt.Portfolio(file, session)
    if not cache:
        rf.prettier_portfolio(prt_loaded, total)


if __name__ == "__main__":
    app()
