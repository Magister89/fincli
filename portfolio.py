"""
Portfolio Class and functions
"""
import json as js
from rich import print as printp
import ticker as tk
import fincli_cache as fcache


class Portfolio:
    """
    Portfolio Class
    """

    def __init__(self, file_path: str, session: fcache.CachedLimiterSession):
        self.portfolio = self.load_portfolio(file_path)
        self.tickers = None
        self.update_status(session)
        self.total_portfolio_value = self.total_value()

    def load_portfolio(self, file_path: str):
        """
        Loads portfolio from json file
        """
        try:
            with open(file_path, encoding="utf-8") as f:
                portfolio = js.load(f)
        except FileNotFoundError:
            printp(
                f'File [bold white]{file_path}[/bold white] [bold red]not found[/bold red]')
        except js.JSONDecodeError:
            printp(f'File [bold white]{file_path}[/bold white] \
                   [bold red]is not a valid JSON[/bold red]')
        self.validate_portfolio_data(portfolio)
        return portfolio

    def validate_portfolio_data(self, portfolio: object):
        """
        JSON portfolio validation
        """
        if not isinstance(portfolio, list):
            raise ValueError("JSON should be a list of objects")

        for item in portfolio:
            if not isinstance(item, dict):
                raise ValueError("Each item should be a dict")
            if "ticker" not in item or "shares" not in item:
                raise ValueError(
                    "Every attribute should be 'ticker' and 'shares'")
            if not isinstance(item["ticker"], str) or not isinstance(item["shares"], int):
                raise ValueError(
                    "Attributes 'ticker' and 'shares' should string and int respectively")

    def update_status(self, session: fcache.CachedLimiterSession):
        """
        Updates portfolio with price and previous close
        """
        tickers = ' '.join(self.get_tickers())
        self.tickers = tk.Tickers(tickers, session=session)
        for ticker in self.portfolio:
            ticker['price'] = round(ticker['shares'] *
                                    self.tickers.get_ticker_fast_info(ticker['ticker'])['lastPrice'], 2)
            ticker['previousClose'] = round(ticker['shares'] *
                                            self.tickers.get_ticker_fast_info(ticker['ticker'])['previousClose'], 2)
            ticker['p&l'] = round(
                ((ticker['price']/ticker['previousClose']) - 1) * 100, 2)

    def total_value(self):
        """
        Returns total portfolio value and profit/loss
        """
        total_value = 0.00
        total_previous = 0.00
        for ticker in self.portfolio:
            total_value += ticker['price']
            total_previous += ticker['previousClose']
        total_pl = round((total_value/total_previous - 1) * 100, 2)
        return [{"ticker": "Portfolio", "shares": 0, "price": round(total_value, 2),
                 "previousClose": round(total_previous, 2), "p&l": total_pl}]

    def get_portfolio(self):
        """
        Returns Portfolio
        """
        return self.portfolio

    def set_portfolio(self, file_path: str, session: fcache.CachedLimiterSession):
        """
        Reloads portfolio
        """
        self.portfolio = self.load_portfolio(file_path)
        self.update_status(session)
        self.total_portfolio_value = self.total_value()

    def get_tickers(self):
        """
        Returns tickers list
        """
        return [ticker['ticker'] for ticker in self.portfolio]

    def get_total_portfolio_value(self):
        """
        Gets total portfolio value and yield
        """
        return self.total_portfolio_value
