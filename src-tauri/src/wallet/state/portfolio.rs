use super::{
    freshness::combine,
    types::{
        BalanceState, FreshnessMetadata, FreshnessStatus, PortfolioState, PriceState, PriceStatus,
    },
};

fn price_to_freshness(source: &str, price_state: &PriceState) -> FreshnessMetadata {
    let status = match (price_state.status, price_state.price_usd) {
        (_, None) | (PriceStatus::Unavailable, _) => FreshnessStatus::Unavailable,
        (PriceStatus::Fresh, Some(_)) => FreshnessStatus::Fresh,
        (PriceStatus::Cached, Some(_)) => FreshnessStatus::Cached,
        (PriceStatus::Stale, Some(_)) => FreshnessStatus::Stale,
        (PriceStatus::Partial, Some(_)) => FreshnessStatus::Partial,
        (PriceStatus::Synthetic, Some(_)) => FreshnessStatus::Partial,
    };

    let failed_sources = if matches!(
        status,
        FreshnessStatus::Partial | FreshnessStatus::Unavailable
    ) {
        vec![source.to_string()]
    } else {
        Vec::new()
    };

    FreshnessMetadata {
        status,
        updated_at: price_state.price_updated_at,
        failed_sources,
    }
}

/// Sum display_amount values weighted by corresponding PriceState.
pub fn aggregate(items: &[(String, BalanceState, PriceState)]) -> PortfolioState {
    let mut component_freshness = Vec::with_capacity(items.len());
    let mut value_usd = 0.0;
    let mut contributed = false;

    for (source, balance_state, price_state) in items {
        let price_freshness = price_to_freshness(source, price_state);
        let component = combine(&[
            (source.clone(), balance_state.freshness.clone()),
            (source.clone(), price_freshness.clone()),
        ]);

        if balance_state.freshness.status != FreshnessStatus::Unavailable {
            if let Some(price_usd) = price_state.price_usd {
                if price_state.status != PriceStatus::Unavailable {
                    value_usd += balance_state.display_amount * price_usd;
                    contributed = true;
                }
            }
        }

        component_freshness.push((source.clone(), component));
    }

    PortfolioState {
        value_usd: contributed.then_some(value_usd),
        value_btc: None,
        freshness: combine(&component_freshness),
    }
}

#[cfg(test)]
mod tests {
    use super::aggregate;
    use crate::wallet::state::{
        price::{from_fetch, synthetic, unavailable},
        types::{
            BalanceState, FreshnessMetadata, FreshnessStatus, PortfolioState, PriceState,
            PriceStatus,
        },
    };

    fn freshness(status: FreshnessStatus, updated_at: Option<i64>) -> FreshnessMetadata {
        FreshnessMetadata {
            status,
            updated_at,
            failed_sources: Vec::new(),
        }
    }

    fn balance(
        display_amount: f64,
        chain_id: Option<&str>,
        freshness: FreshnessMetadata,
    ) -> BalanceState {
        BalanceState {
            raw_amount: format!("{display_amount}"),
            display_amount,
            chain_id: chain_id.map(str::to_string),
            freshness,
        }
    }

    // M-BS-1
    #[test]
    fn balance_state_round_trips_for_bitcoin_and_evm_shapes() {
        let bitcoin = balance(0.5, None, freshness(FreshnessStatus::Fresh, Some(100)));
        let evm = balance(
            2.0,
            Some("ethereum"),
            freshness(FreshnessStatus::Fresh, Some(100)),
        );

        let bitcoin_wire = serde_json::to_string(&bitcoin).unwrap();
        let evm_wire = serde_json::to_string(&evm).unwrap();

        let bitcoin_round_trip: BalanceState = serde_json::from_str(&bitcoin_wire).unwrap();
        let evm_round_trip: BalanceState = serde_json::from_str(&evm_wire).unwrap();

        assert_eq!(bitcoin_round_trip.chain_id, None);
        assert_eq!(evm_round_trip.chain_id.as_deref(), Some("ethereum"));
    }

    // M-BS-2
    #[test]
    fn stale_balance_remains_distinct_from_cached_after_round_trip() {
        let stale = balance(1.0, None, freshness(FreshnessStatus::Stale, Some(100)));
        let cached = balance(1.0, None, freshness(FreshnessStatus::Cached, Some(100)));

        let stale_round_trip: BalanceState =
            serde_json::from_str(&serde_json::to_string(&stale).unwrap()).unwrap();
        let cached_round_trip: BalanceState =
            serde_json::from_str(&serde_json::to_string(&cached).unwrap()).unwrap();

        assert_eq!(stale_round_trip.freshness.status, FreshnessStatus::Stale);
        assert_eq!(cached_round_trip.freshness.status, FreshnessStatus::Cached);
        assert_ne!(
            stale_round_trip.freshness.status,
            cached_round_trip.freshness.status
        );
    }

    // M-PT-1
    #[test]
    fn unavailable_component_never_yields_a_fresh_portfolio() {
        // PortfolioState Unavailable Fresh
        let items = vec![
            (
                "bitcoin".to_string(),
                balance(1.0, None, freshness(FreshnessStatus::Fresh, Some(200))),
                from_fetch(100_000.0, "coingecko", 200, 205, 30, 120),
            ),
            (
                "ethereum".to_string(),
                balance(
                    2.0,
                    Some("ethereum"),
                    freshness(FreshnessStatus::Unavailable, None),
                ),
                from_fetch(2_000.0, "coingecko", 200, 205, 30, 120),
            ),
        ];

        let portfolio: PortfolioState = aggregate(&items);

        assert!(matches!(
            portfolio.freshness.status,
            FreshnessStatus::Partial | FreshnessStatus::Unavailable
        ));
        assert_ne!(portfolio.freshness.status, FreshnessStatus::Fresh);
        assert!(portfolio
            .freshness
            .failed_sources
            .iter()
            .any(|source| source == "ethereum"));
        assert_eq!(portfolio.value_usd, Some(100_000.0));
    }

    // M-PT-2
    #[test]
    fn all_fresh_inputs_stay_fresh_and_non_fresh_inputs_degrade_portfolio() {
        let fresh_items = vec![
            (
                "bitcoin".to_string(),
                balance(1.0, None, freshness(FreshnessStatus::Fresh, Some(200))),
                from_fetch(100_000.0, "coingecko", 200, 205, 30, 120),
            ),
            (
                "ethereum".to_string(),
                balance(
                    2.0,
                    Some("ethereum"),
                    freshness(FreshnessStatus::Fresh, Some(201)),
                ),
                from_fetch(2_000.0, "coingecko", 201, 205, 30, 120),
            ),
        ];
        let stale_items = vec![
            (
                "bitcoin".to_string(),
                balance(1.0, None, freshness(FreshnessStatus::Fresh, Some(200))),
                PriceState {
                    price_usd: Some(100_000.0),
                    price_source: Some("coingecko".to_string()),
                    price_updated_at: Some(50),
                    status: PriceStatus::Stale,
                },
            ),
            (
                "ethereum".to_string(),
                balance(
                    2.0,
                    Some("ethereum"),
                    freshness(FreshnessStatus::Fresh, Some(201)),
                ),
                from_fetch(2_000.0, "coingecko", 201, 205, 30, 120),
            ),
        ];
        let synthetic_items = vec![
            (
                "usdc".to_string(),
                balance(
                    10.0,
                    Some("ethereum"),
                    freshness(FreshnessStatus::Fresh, Some(205)),
                ),
                synthetic(1.0, "synthetic-stablecoin", 205),
            ),
            (
                "bitcoin".to_string(),
                balance(1.0, None, freshness(FreshnessStatus::Fresh, Some(200))),
                from_fetch(100_000.0, "coingecko", 200, 205, 30, 120),
            ),
        ];
        let cached_balance_items = vec![(
            "bitcoin".to_string(),
            balance(1.0, None, freshness(FreshnessStatus::Cached, None)),
            from_fetch(100_000.0, "coingecko", 200, 205, 30, 120),
        )];

        let cached_price_items = vec![(
            "bitcoin".to_string(),
            balance(1.0, None, freshness(FreshnessStatus::Fresh, Some(200))),
            PriceState {
                price_usd: Some(100_000.0),
                price_source: Some("coingecko".to_string()),
                price_updated_at: Some(200),
                status: PriceStatus::Cached,
            },
        )];
        let partial_price_items = vec![(
            "bitcoin".to_string(),
            balance(1.0, None, freshness(FreshnessStatus::Fresh, Some(200))),
            PriceState {
                price_usd: Some(100_000.0),
                price_source: Some("coingecko".to_string()),
                price_updated_at: Some(200),
                status: PriceStatus::Partial,
            },
        )];

        let fresh_portfolio = aggregate(&fresh_items);
        let stale_portfolio = aggregate(&stale_items);
        let synthetic_portfolio = aggregate(&synthetic_items);
        let cached_portfolio = aggregate(&cached_balance_items);
        let cached_price_portfolio = aggregate(&cached_price_items);
        let partial_price_portfolio = aggregate(&partial_price_items);

        assert_eq!(fresh_portfolio.freshness.status, FreshnessStatus::Fresh);
        assert_eq!(stale_portfolio.freshness.status, FreshnessStatus::Stale);
        assert_eq!(
            synthetic_portfolio.freshness.status,
            FreshnessStatus::Partial
        );
        assert_eq!(cached_portfolio.freshness.status, FreshnessStatus::Cached);
        assert_eq!(
            cached_price_portfolio.freshness.status,
            FreshnessStatus::Cached
        );
        assert_eq!(
            partial_price_portfolio.freshness.status,
            FreshnessStatus::Partial
        );
        assert!(synthetic_portfolio
            .freshness
            .failed_sources
            .iter()
            .any(|source| source == "usdc"));
    }

    #[test]
    fn synthetic_prices_contribute_but_unavailable_prices_do_not() {
        let items = vec![
            (
                "usdc".to_string(),
                balance(
                    10.0,
                    Some("ethereum"),
                    freshness(FreshnessStatus::Fresh, Some(205)),
                ),
                synthetic(1.0, "synthetic-stablecoin", 205),
            ),
            (
                "doge".to_string(),
                balance(
                    5.0,
                    Some("dogecoin"),
                    freshness(FreshnessStatus::Fresh, Some(205)),
                ),
                unavailable(),
            ),
        ];

        let portfolio = aggregate(&items);

        assert_eq!(portfolio.value_usd, Some(10.0));
        assert_eq!(portfolio.value_btc, None);
        assert!(matches!(
            portfolio.freshness.status,
            FreshnessStatus::Partial | FreshnessStatus::Unavailable
        ));
        assert!(portfolio
            .freshness
            .failed_sources
            .iter()
            .any(|source| source == "doge"));
    }
}
