use super::{
    freshness::classify_age,
    types::{PriceState, PriceStatus},
};

/// Construct a PriceState from a fetched price tuple + age classifier.
pub fn from_fetch(
    price_usd: f64,
    source: &str,
    fetched_at: i64,
    now: i64,
    fresh_within_secs: i64,
    stale_after_secs: i64,
) -> PriceState {
    let freshness = classify_age(Some(fetched_at), now, fresh_within_secs, stale_after_secs);

    let status = match freshness {
        super::types::FreshnessStatus::Fresh => PriceStatus::Fresh,
        super::types::FreshnessStatus::Cached => PriceStatus::Cached,
        super::types::FreshnessStatus::Stale => PriceStatus::Stale,
        super::types::FreshnessStatus::Partial => PriceStatus::Partial,
        super::types::FreshnessStatus::Unavailable => PriceStatus::Unavailable,
    };

    PriceState {
        price_usd: Some(price_usd),
        price_source: Some(source.to_string()),
        price_updated_at: Some(fetched_at),
        status,
    }
}

/// Construct a PriceState representing an unavailable price (last resort).
pub fn unavailable() -> PriceState {
    PriceState {
        price_usd: None,
        price_source: None,
        price_updated_at: None,
        status: PriceStatus::Unavailable,
    }
}

/// Construct a PriceState from a synthetic source (see ADR-0003).
pub fn synthetic(price_usd: f64, source: &str, now: i64) -> PriceState {
    PriceState {
        price_usd: Some(price_usd),
        price_source: Some(source.to_string()),
        price_updated_at: Some(now),
        status: PriceStatus::Synthetic,
    }
}

/// Predicate required by ADR-0003.
pub fn is_usable_fresh(state: &PriceState) -> bool {
    matches!(state.status, PriceStatus::Fresh)
}

#[cfg(test)]
mod tests {
    use super::{from_fetch, is_usable_fresh, synthetic, unavailable};
    use crate::wallet::state::types::{PriceState, PriceStatus};

    // M-PS-1
    #[test]
    fn price_status_variants_round_trip_with_snake_case() {
        let cases = [
            (PriceStatus::Fresh, "\"fresh\""),
            (PriceStatus::Cached, "\"cached\""),
            (PriceStatus::Stale, "\"stale\""),
            (PriceStatus::Partial, "\"partial\""),
            (PriceStatus::Unavailable, "\"unavailable\""),
            (PriceStatus::Synthetic, "\"synthetic\""),
        ];

        for (status, wire) in cases {
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, wire);

            let round_trip: PriceStatus = serde_json::from_str(wire).unwrap();
            assert_eq!(round_trip, status);
        }
    }

    // M-PS-2
    #[test]
    fn synthetic_is_not_usable_for_price_checks() {
        // assert PriceStatus::Synthetic per ADR-0003
        // ADR-0003: synthetic prices are not fresh prices.
        let state = synthetic(1.0, "synthetic-stablecoin", 1_713_000_000);

        assert_eq!(state.status, PriceStatus::Synthetic);
        assert!(!is_usable_fresh(&state));
    }

    // M-PS-3
    #[test]
    fn unavailable_price_round_trips_and_formats_without_panicking() {
        let state = unavailable();
        let serialized = serde_json::to_string(&state).unwrap();
        let round_trip: PriceState = serde_json::from_str(&serialized).unwrap();

        assert_eq!(round_trip.status, PriceStatus::Unavailable);
        assert_eq!(round_trip.price_usd, None);
        assert_eq!(round_trip.price_source, None);
        assert_eq!(round_trip.price_updated_at, None);
        assert!(!format!("{:?}", round_trip).is_empty());
    }

    #[test]
    fn from_fetch_marks_old_prices_as_stale() {
        // PriceStatus::Stale
        let state = from_fetch(42.0, "coingecko", 100, 200, 30, 90);

        assert_eq!(state.status, PriceStatus::Stale);
        assert_eq!(state.price_usd, Some(42.0));
        assert_eq!(state.price_source.as_deref(), Some("coingecko"));
        assert_eq!(state.price_updated_at, Some(100));
    }

    #[test]
    fn from_fetch_marks_recent_prices_as_fresh() {
        let state = from_fetch(42.0, "coingecko", 190, 200, 30, 90);

        assert_eq!(state.status, PriceStatus::Fresh);
        assert!(is_usable_fresh(&state));
    }
}
