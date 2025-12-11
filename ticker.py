"""
Ticker Classes and Functions
"""
import yfinance as yf


class Tickers:
    """
    Tickers Class
    """

    def __init__(self, tickers: str):
        self.tickers = tickers
        self.tickers_data = yf.Tickers(tickers)

    def get_tickers(self):
        """
        Returns tickers
        """
        return self.tickers

    def set_tickers(self, tickers: str):
        """
        Resets tickers
        """
        self.tickers = tickers
        self.tickers_data = yf.Tickers(tickers)

    def get_tickers_list(self):
        """
        Returns tickers list
        """
        return self.tickers.split()

    def get_tickers_data(self):
        """
        Returns tickers data from yfinance
        """
        return self.tickers_data

    def get_ticker_fast_info(self, tkr: str):
        """
        Returns fast_info object
        """
        return self.tickers_data.tickers[tkr].fast_info

    def get_ticker_info(self, tkr: str):
        """
        Returns info object
        """
        return self.tickers_data.tickers[tkr].info


class Ticker:
    """
    Ticker Class
    """

    def __init__(self, ticker: str):
        self.ticker = ticker
        self.ticker_data = yf.Ticker(ticker)

    def get_ticker(self):
        """
        Returns ticker
        """
        return self.ticker

    def set_ticker(self, ticker: str):
        """
        Resets ticker
        """
        self.ticker = ticker
        self.ticker_data = yf.Ticker(ticker)

    def get_ticker_data(self):
        """
        Returns ticker data
        """
        return self.ticker_data

    def get_ticker_fast_info(self):
        """
        Returns fast info object
        """
        return self.ticker_data.fast_info

    def get_ticker_info(self):
        """
        Returns info object
        """
        return self.ticker_data.info
