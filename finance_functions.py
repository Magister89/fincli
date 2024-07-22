"""
Functions to get info from Yahoo Finance
"""
from typing import Optional
import json as js
import yfinance as yf
from rich import print as printp
import typer_functions as tf

def validate_portfolio_data(portfolio: object):
    """
    JSON portfolio validation
    """
    if not isinstance(portfolio, list):
        raise ValueError("JSON should be a list of objects")

    for item in portfolio:
        if not isinstance(item, dict):
            raise ValueError("Each item should be a dict")
        if "ticker" not in item or "shares" not in item:
            raise ValueError("Every attribute should be 'ticker' and 'shares'")
        if not isinstance(item["ticker"], str) or not isinstance(item["shares"], int):
            raise ValueError("Attributes 'ticker' and 'shares' should string and int respectively")

def get_ticker(ticker: str):
    """
    Returns selected ticker
    """
    return yf.Ticker(ticker)

def get_tickers(tickers: list):
    """
    Returns selected tickers
    """
    return yf.Tickers(tickers)

def get_portfolio(file_path: str):
    """
    Gets json portfolio
    """
    try:
        with open(file_path, encoding="utf-8") as f:
            portfolio = js.load(f)
    except FileNotFoundError:
        printp(f'File [bold white]{file_path}[/bold white] [bold red]not found[/bold red]')
    except js.JSONDecodeError:
        printp(f'File [bold white]{file_path}[/bold white] \
               [bold red]is not a valid JSON[/bold red]')
    validate_portfolio_data(portfolio)
    return portfolio

def get_ticker_info_price(tkr: str, info: Optional[str], attribute: str):
    """
    Gets ticker info or attribute value
    """
    try:
        if info:
            info_obj = get_ticker(tkr).fast_info
            tf.prettier_info(info_obj)
        else:
            attribute_value = get_ticker(tkr).fast_info[attribute]
            attribute_value = round(attribute_value, 2) if isinstance(attribute_value, float) else attribute_value
            printp(f"{attribute} for [bold green]{tkr}[/bold green] is [bold]{attribute_value}[/bold]")
    except KeyError:
        printp(f"Ticker [bold white]{tkr}[/bold white] [bold red]is not valid[/bold red]")


def get_portfolio_value(file_path: str, total: bool):
    """
    Loads portfolio from json file and adds info from Yahoo Finance
    """
    json = get_portfolio(file_path)
    tickers_list = ' '.join([element['ticker'] for element in json])
    yf_tickers = get_tickers(tickers_list)
    total_value = 0.00
    total_previous = 0.00
    for element in json:
        try:
            if total:
                total_value += round(element['shares'] *
                                yf_tickers.tickers[element['ticker']].fast_info['lastPrice'], 2)
                total_previous += round(element['shares'] *
                                yf_tickers.tickers[element['ticker']].fast_info['previousClose'], 2)
            else:
                element['price'] = round(element['shares'] *
                                yf_tickers.tickers[element['ticker']].fast_info['lastPrice'], 2)
                element['previousClose'] = round(element['shares'] *
                                yf_tickers.tickers[element['ticker']].fast_info['previousClose'], 2)
                element['yield'] = round(((element['price']/element['previousClose']) - 1) * 100, 2)
        except KeyError:
            printp(f"Ticker [bold white]{element['ticker']}[/bold white] \
                   [bold red]is not valid[/bold red]")
    if total:
        total_yield = round((total_value/total_previous - 1) * 100, 2)
        json = [{"ticker": "Portfolio", "shares": 0, "price": round(total_value, 2),
                 "previousClose": round(total_previous, 2), "yield": total_yield}]        
    return json


def portfolio_print(file_path: str, total: bool):
    """
    Prettier printer
    """
    tf.prettier_portfolio(get_portfolio_value(file_path, total), total)
