use std::collections::BTreeSet;

use super::types::{FreshnessMetadata, FreshnessStatus};

/// Classify age into Fresh / Cached / Stale given fresh/stale thresholds
/// in seconds. None updated_at -> Cached.
pub fn classify_age(
    updated_at: Option<i64>,
    now: i64,
    fresh_within_secs: i64,
    stale_after_secs: i64,
) -> FreshnessStatus {
    let Some(updated_at) = updated_at else {
        return FreshnessStatus::Cached;
    };

    let age_secs = now.saturating_sub(updated_at);

    if age_secs <= fresh_within_secs {
        FreshnessStatus::Fresh
    } else if age_secs >= stale_after_secs {
        FreshnessStatus::Stale
    } else {
        FreshnessStatus::Cached
    }
}

/// Combine component freshness into aggregate freshness for a portfolio
/// or multi-source view.
pub fn combine(components: &[(String, FreshnessMetadata)]) -> FreshnessMetadata {
    if components.is_empty() {
        return FreshnessMetadata {
            status: FreshnessStatus::Unavailable,
            updated_at: None,
            failed_sources: Vec::new(),
        };
    }

    let mut failed_sources = BTreeSet::new();
    let mut aggregate_status = FreshnessStatus::Fresh;
    let mut min_updated_at: Option<i64> = None;
    let mut saw_unknown_updated_at = false;

    for (source, metadata) in components {
        for failed_source in &metadata.failed_sources {
            failed_sources.insert(failed_source.clone());
        }

        match metadata.updated_at {
            Some(updated_at) => {
                min_updated_at = Some(match min_updated_at {
                    Some(current_min) => current_min.min(updated_at),
                    None => updated_at,
                });
            }
            None => saw_unknown_updated_at = true,
        }

        match metadata.status {
            FreshnessStatus::Unavailable => {
                aggregate_status = FreshnessStatus::Unavailable;
                failed_sources.insert(source.clone());
            }
            FreshnessStatus::Partial if aggregate_status != FreshnessStatus::Unavailable => {
                aggregate_status = FreshnessStatus::Partial;
            }
            FreshnessStatus::Stale
                if !matches!(
                    aggregate_status,
                    FreshnessStatus::Unavailable | FreshnessStatus::Partial
                ) =>
            {
                aggregate_status = FreshnessStatus::Stale;
            }
            FreshnessStatus::Cached
                if matches!(aggregate_status, FreshnessStatus::Fresh) =>
            {
                aggregate_status = FreshnessStatus::Cached;
            }
            FreshnessStatus::Fresh | FreshnessStatus::Cached | FreshnessStatus::Stale => {}
            FreshnessStatus::Partial => {}
        }
    }

    FreshnessMetadata {
        status: aggregate_status,
        updated_at: if saw_unknown_updated_at {
            None
        } else {
            min_updated_at
        },
        failed_sources: failed_sources.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_age, combine};
    use crate::wallet::state::types::{FreshnessMetadata, FreshnessStatus};

    // M-FS-1
    #[test]
    fn freshness_status_variants_round_trip_with_snake_case() {
        let cases = [
            (FreshnessStatus::Fresh, "\"fresh\""),
            (FreshnessStatus::Cached, "\"cached\""),
            (FreshnessStatus::Stale, "\"stale\""),
            (FreshnessStatus::Unavailable, "\"unavailable\""),
            (FreshnessStatus::Partial, "\"partial\""),
        ];

        for (status, wire) in cases {
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, wire);

            let round_trip: FreshnessStatus = serde_json::from_str(wire).unwrap();
            assert_eq!(round_trip, status);
        }
    }

    // M-FS-2
    #[test]
    fn partial_freshness_metadata_round_trips_with_failed_sources() { // FreshnessStatus::Partial failed_sources
        let metadata = FreshnessMetadata {
            status: FreshnessStatus::Partial,
            updated_at: Some(1_713_000_000),
            failed_sources: vec!["x".to_string(), "y".to_string()],
        };

        let serialized = serde_json::to_string(&metadata).unwrap();
        assert!(serialized.contains(r#""failed_sources":["x","y"]"#));

        let round_trip: FreshnessMetadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(round_trip.status, FreshnessStatus::Partial);
        assert_eq!(round_trip.updated_at, Some(1_713_000_000));
        assert_eq!(round_trip.failed_sources, vec!["x", "y"]);
    }

    #[test]
    fn partial_shape_permits_empty_failed_sources_when_explicitly_asserted() {
        let metadata = FreshnessMetadata {
            status: FreshnessStatus::Partial,
            updated_at: None,
            failed_sources: Vec::new(),
        };

        let serialized = serde_json::to_string(&metadata).unwrap();
        let round_trip: FreshnessMetadata = serde_json::from_str(&serialized).unwrap();

        assert_eq!(round_trip.status, FreshnessStatus::Partial);
        assert!(round_trip.failed_sources.is_empty());
    }

    #[test]
    fn classify_age_uses_cached_for_unknown_and_respects_threshold_boundaries() {
        assert_eq!(classify_age(None, 200, 30, 60), FreshnessStatus::Cached);
        assert_eq!(classify_age(Some(190), 200, 10, 30), FreshnessStatus::Fresh);
        assert_eq!(classify_age(Some(180), 200, 10, 30), FreshnessStatus::Cached);
        assert_eq!(classify_age(Some(170), 200, 10, 30), FreshnessStatus::Stale);
    }

    #[test]
    fn combine_all_fresh_input_returns_fresh() {
        let combined = combine(&[
            (
                "bitcoin".to_string(),
                FreshnessMetadata {
                    status: FreshnessStatus::Fresh,
                    updated_at: Some(200),
                    failed_sources: Vec::new(),
                },
            ),
            (
                "ethereum".to_string(),
                FreshnessMetadata {
                    status: FreshnessStatus::Fresh,
                    updated_at: Some(190),
                    failed_sources: Vec::new(),
                },
            ),
        ]);

        assert_eq!(combined.status, FreshnessStatus::Fresh);
        assert_eq!(combined.updated_at, Some(190));
        assert!(combined.failed_sources.is_empty());
    }

    #[test]
    fn combine_single_stale_component_returns_stale() {
        let combined = combine(&[
            (
                "bitcoin".to_string(),
                FreshnessMetadata {
                    status: FreshnessStatus::Fresh,
                    updated_at: Some(200),
                    failed_sources: Vec::new(),
                },
            ),
            (
                "ethereum".to_string(),
                FreshnessMetadata {
                    status: FreshnessStatus::Stale,
                    updated_at: Some(180),
                    failed_sources: Vec::new(),
                },
            ),
        ]);

        assert_eq!(combined.status, FreshnessStatus::Stale);
        assert_eq!(combined.updated_at, Some(180));
        assert!(combined.failed_sources.is_empty());
    }

    #[test]
    fn combine_single_cached_component_returns_cached() {
        let combined = combine(&[
            (
                "bitcoin".to_string(),
                FreshnessMetadata {
                    status: FreshnessStatus::Fresh,
                    updated_at: Some(200),
                    failed_sources: Vec::new(),
                },
            ),
            (
                "ethereum".to_string(),
                FreshnessMetadata {
                    status: FreshnessStatus::Cached,
                    updated_at: Some(180),
                    failed_sources: Vec::new(),
                },
            ),
        ]);

        assert_eq!(combined.status, FreshnessStatus::Cached);
        assert_eq!(combined.updated_at, Some(180));
        assert!(combined.failed_sources.is_empty());
    }

    #[test]
    fn combine_prefers_worst_status_and_unions_failed_sources() {
        let combined = combine(&[
            (
                "bitcoin".to_string(),
                FreshnessMetadata {
                    status: FreshnessStatus::Fresh,
                    updated_at: Some(200),
                    failed_sources: Vec::new(),
                },
            ),
            (
                "ethereum".to_string(),
                FreshnessMetadata {
                    status: FreshnessStatus::Partial,
                    updated_at: Some(180),
                    failed_sources: vec!["rpc".to_string()],
                },
            ),
            (
                "coingecko".to_string(),
                FreshnessMetadata {
                    status: FreshnessStatus::Unavailable,
                    updated_at: None,
                    failed_sources: vec!["cache".to_string()],
                },
            ),
        ]);

        assert_eq!(combined.status, FreshnessStatus::Unavailable);
        assert_eq!(combined.updated_at, None);
        assert_eq!(combined.failed_sources, vec!["cache", "coingecko", "rpc"]);
    }

    #[test]
    fn combine_empty_input_is_unavailable_with_no_failed_sources() {
        let combined = combine(&[]);

        assert_eq!(combined.status, FreshnessStatus::Unavailable);
        assert_eq!(combined.updated_at, None);
        assert!(combined.failed_sources.is_empty());
    }
}