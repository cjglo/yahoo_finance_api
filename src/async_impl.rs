use crate::quotes::YQuoteSummary;
use search_result::YOptionChain;

use super::*;

impl YahooConnector {
    /// Retrieve the quotes of the last day for the given ticker
    pub async fn get_latest_quotes(
        &self,
        ticker: &str,
        interval: &str,
    ) -> Result<YResponse, YahooError> {
        self.get_quote_range(ticker, interval, "1mo").await
    }

    /// Retrieve the quote history for the given ticker form date start to end (inclusive), if available
    pub async fn get_quote_history(
        &self,
        ticker: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
    ) -> Result<YResponse, YahooError> {
        self.get_quote_history_interval(ticker, start, end, "1d")
            .await
    }

    /// Retrieve quotes for the given ticker for an arbitrary range
    pub async fn get_quote_range(
        &self,
        ticker: &str,
        interval: &str,
        range: &str,
    ) -> Result<YResponse, YahooError> {
        let url: String = format!(
            YCHART_RANGE_QUERY!(),
            url = self.url,
            symbol = ticker,
            interval = interval,
            range = range
        );
        YResponse::from_json(self.send_request(&url).await?)
    }

    /// Retrieve the quote history for the given ticker form date start to end (inclusive), if available; specifying the interval of the ticker.
    pub async fn get_quote_history_interval(
        &self,
        ticker: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
        interval: &str,
    ) -> Result<YResponse, YahooError> {
        let url = format!(
            YCHART_PERIOD_QUERY!(),
            url = self.url,
            symbol = ticker,
            start = start.unix_timestamp(),
            end = end.unix_timestamp(),
            interval = interval,
        );
        YResponse::from_json(self.send_request(&url).await?)
    }

    /// Retrieve the quote history for the given ticker form date start to end (inclusive) and optionally before and after regular trading hours, if available; specifying the interval of the ticker.
    pub async fn get_quote_history_interval_prepost(
        &self,
        ticker: &str,
        start: OffsetDateTime,
        end: OffsetDateTime,
        interval: &str,
        prepost: bool,
    ) -> Result<YResponse, YahooError> {
        let url = format!(
            YCHART_PERIOD_QUERY_PRE_POST!(),
            url = self.url,
            symbol = ticker,
            start = start.unix_timestamp(),
            end = end.unix_timestamp(),
            interval = interval,
            prepost = prepost,
        );
        YResponse::from_json(self.send_request(&url).await?)
    }

    /// Retrieve the quote history for the given ticker for a given period and ticker interval and optionally before and after regular trading hours
    pub async fn get_quote_period_interval(
        &self,
        ticker: &str,
        period: &str,
        interval: &str,
        prepost: bool,
    ) -> Result<YResponse, YahooError> {
        let url = format!(
            YCHART_PERIOD_INTERVAL_QUERY!(),
            url = self.url,
            symbol = ticker,
            period = period,
            interval = interval,
            prepost = prepost,
        );
        YResponse::from_json(self.send_request(&url).await?)
    }

    /// Retrieve the list of quotes found searching a given name
    pub async fn search_ticker_opt(&self, name: &str) -> Result<YSearchResultOpt, YahooError> {
        let url = format!(YTICKER_QUERY!(), url = self.search_url, name = name);
        YSearchResultOpt::from_json(self.send_request(&url).await?)
    }

    /// Retrieve the list of quotes found searching a given name
    pub async fn search_ticker(&self, name: &str) -> Result<YSearchResult, YahooError> {
        let result = self.search_ticker_opt(name).await?;
        Ok(YSearchResult::from_opt(&result))
    }

    /// Get list for options for a given name
    pub async fn search_options(&self, name: &str) -> Result<YOptionChain, YahooError> {
        let url = format!("https://query2.finance.yahoo.com/v6/finance/options/{name}");
        let resp = self.client.get(url).send().await?;
        let resp = resp.json::<YOptionChain>().await?;

        Ok(resp)
    }

    // Get symbol metadata
    pub async fn get_ticker_info(symbol: &str) -> Result<YQuoteSummary, YahooError> {
        let get_cookie_resp = reqwest::get(Y_GET_COOKIE_URL).await.unwrap();
        let cookie = get_cookie_resp
            .headers()
            .get(Y_COOKIE_REQUEST_HEADER)
            .unwrap()
            .to_str()
            .unwrap();
        let jar_for_cookie = std::sync::Arc::new(reqwest::cookie::Jar::default());
        jar_for_cookie.add_cookie_str(cookie, &reqwest::Url::parse(Y_GET_CRUMB_URL).unwrap());
        let intermediate_client_for_cookie = Client::builder()
            .user_agent(USER_AGENT)
            .cookie_provider(jar_for_cookie)
            .build()
            .unwrap();

        let crumb = intermediate_client_for_cookie
            .get(reqwest::Url::parse(Y_GET_CRUMB_URL).unwrap())
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        let jar_for_info = std::sync::Arc::new(reqwest::cookie::Jar::default());
        let url_for_information = reqwest::Url::parse(
            &(format!(YQUOTE_SUMMARY_QUERY!(), symbol = symbol, crumb = crumb)),
        );
        jar_for_info.add_cookie_str(cookie, &url_for_information.clone().unwrap());

        let client_for_info = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .cookie_provider(jar_for_info)
            .build()
            .unwrap();
        let resp = client_for_info
            .get(url_for_information.unwrap())
            .send()
            .await
            .unwrap();
        let result = resp.json().await.unwrap();
        Ok(result)
    }

    /// Send request to yahoo! finance server and transform response to JSON value
    async fn send_request(&self, url: &str) -> Result<serde_json::Value, YahooError> {
        let resp = self.client.get(url).send().await?;

        match resp.status() {
            StatusCode::OK => Ok(resp.json().await?),
            status => Err(YahooError::FetchFailed(format!("{}", status))),
        }
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn test_get_single_quote() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("HNL.DE", "1d")).unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_strange_api_responses() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2019-07-03 0:00:00.00 UTC);
        let end = datetime!(2020-07-04 23:59:59.99 UTC);

        let resp = tokio_test::block_on(provider.get_quote_history("IBM", start, end)).unwrap();

        assert_eq!(&resp.chart.result[0].meta.symbol, "IBM");
        assert_eq!(&resp.chart.result[0].meta.data_granularity, "1d");
        assert_eq!(
            &resp.chart.result[0].meta.first_trade_date,
            &Some(-252322200)
        );

        let _ = resp.last_quote().unwrap();
    }

    #[test]
    #[should_panic(expected = "DeserializeFailed")]
    fn test_api_responses_missing_fields() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("BF.B", "1m")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "BF.B");
        assert_eq!(&response.chart.result[0].meta.range, "1d");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1m");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_quote_history() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2020-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let resp = tokio_test::block_on(provider.get_quote_history("AAPL", start, end));
        if resp.is_ok() {
            let resp = resp.unwrap();
            assert_eq!(resp.chart.result[0].timestamp.len(), 21);
            let quotes = resp.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
        }
    }

    #[test]
    fn test_get_quote_range() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("HNL.DE", "1d", "1mo")).unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "HNL.DE");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_get_metadata() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("HNL.DE", "1d", "1mo")).unwrap();
        let metadata = response.metadata().unwrap();
        assert_eq!(metadata.symbol, "HNL.DE");
    }

    #[test]
    fn test_get() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2019-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let response =
            tokio_test::block_on(provider.get_quote_history_interval("AAPL", start, end, "1mo"))
                .unwrap();
        assert_eq!(&response.chart.result[0].timestamp.len(), &13);
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1mo");
        let quotes = response.quotes().unwrap();
        assert_eq!(quotes.len(), 13usize);
    }

    #[test]
    fn test_large_volume() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("BTC-USD", "1d", "5d")).unwrap();
        let quotes = response.quotes().unwrap();
        assert!(quotes.len() > 0usize);
    }

    #[test]
    fn test_search_ticker() {
        let provider = YahooConnector::new().unwrap();
        let resp = tokio_test::block_on(provider.search_ticker("Apple")).unwrap();

        assert_eq!(resp.count, 15);
        let mut apple_found = false;
        for item in resp.quotes {
            if item.exchange == "NMS" && item.symbol == "AAPL" && item.short_name == "Apple Inc." {
                apple_found = true;
                break;
            }
        }
        assert!(apple_found)
    }

    #[test]
    fn search_options() {
        let provider = YahooConnector::new().unwrap();
        let resp = tokio_test::block_on(provider.search_options("AAPL"));

        assert!(resp.is_ok());
    }

    #[test]
    fn test_mutual_fund_history() {
        let provider = YahooConnector::new().unwrap();

        let start = datetime!(2020-01-01 0:00:00.00 UTC);
        let end = datetime!(2020-01-31 23:59:59.99 UTC);

        let resp = tokio_test::block_on(provider.get_quote_history("VTSAX", start, end));
        if resp.is_ok() {
            let resp = resp.unwrap();
            assert_eq!(resp.chart.result[0].timestamp.len(), 21);
            let quotes = resp.quotes().unwrap();
            assert_eq!(quotes.len(), 21);
        }
    }

    #[test]
    fn test_mutual_fund_latest() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("VTSAX", "1d")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "VTSAX");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_mutual_fund_latest_with_null_first_trade_date() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_latest_quotes("SIWA.F", "1d")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "SIWA.F");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let _ = response.last_quote().unwrap();
    }

    #[test]
    fn test_mutual_fund_range() {
        let provider = YahooConnector::new().unwrap();
        let response =
            tokio_test::block_on(provider.get_quote_range("VTSAX", "1d", "1mo")).unwrap();
        assert_eq!(&response.chart.result[0].meta.symbol, "VTSAX");
        assert_eq!(&response.chart.result[0].meta.range, "1mo");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
    }

    #[test]
    fn test_mutual_fund_capital_gains() {
        let provider = YahooConnector::new().unwrap();
        let response = tokio_test::block_on(provider.get_quote_range("AMAGX", "1d", "5y")).unwrap();

        assert_eq!(&response.chart.result[0].meta.symbol, "AMAGX");
        assert_eq!(&response.chart.result[0].meta.range, "5y");
        assert_eq!(&response.chart.result[0].meta.data_granularity, "1d");
        let capital_gains = response.capital_gains().unwrap();
        assert!(capital_gains.len() > 0usize);
    }

    #[test]
    fn test_get_ticker_info() {
        let result = tokio_test::block_on(YahooConnector::get_ticker_info("AAPL"));

        assert!(result.is_ok());
        let quote_summary = result.unwrap().quote_summary;
        assert!("Cupertino" == quote_summary.result[0].asset_profile.city); // Testing it retrieved info, hard coded but shouldn't change anytime soon
    }
}
