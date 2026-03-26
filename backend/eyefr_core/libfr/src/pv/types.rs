use chrono::{DateTime, SecondsFormat, Utc};

use crate::{
    pv::pv_grpc::{identity_grpc::identity, proc_grpc::processor},
    types::{BoundingBox, Face, Liveness, Point, PossibleMatch, Template},
    utils,
};

pub(crate) fn possible_matches_from_lookup(
    lookup: &identity::LookupIdentity,
) -> Vec<PossibleMatch> {
    let mut possible_matches: Vec<PossibleMatch> = lookup
        .matches
        .iter()
        .filter_map(|match_item| {
            let identity = match_item.identity.as_ref()?;
            let score = utils::roundf32(match_item.score, 5);
            let mut possible_match = PossibleMatch::new(identity.id.clone(), score);
            possible_match.ext_id = identity.external_id.clone();
            Some(possible_match)
        })
        .collect();

    if possible_matches.len() > 1 {
        possible_matches.sort_by(|a, b| {
            a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal).reverse()
        });
    }

    possible_matches
}

impl From<processor::Face> for Face {
    fn from(pv_face: processor::Face) -> Self {
        let bbox = pv_face.bounding_box.as_ref().map(|bb| BoundingBox {
            origin: Point {
                x: bb.origin.as_ref().map_or(0.0, |point| point.x.floor()),
                y: bb.origin.as_ref().map_or(0.0, |point| point.y.floor()),
            },
            width: bb.width.round(),
            height: bb.height.round(),
        });

        let liveness = to_liveness(pv_face.liveness.as_ref(), pv_face.liveness_validness.as_ref());

        let template =
            (!pv_face.embedding.is_empty()).then(|| Template { embedding: pv_face.embedding });

        Self {
            bbox,
            acceptability: Some(pv_face.acceptability),
            quality: Some(pv_face.quality),
            mask: Some(pv_face.mask),
            template,
            liveness,
        }
    }
}

impl From<&processor::Face> for Face {
    fn from(pv_face: &processor::Face) -> Self {
        let bbox = pv_face.bounding_box.as_ref().map(|bb| BoundingBox {
            origin: Point {
                x: bb.origin.as_ref().map_or(0.0, |point| point.x.floor()),
                y: bb.origin.as_ref().map_or(0.0, |point| point.y.floor()),
            },
            width: bb.width.round(),
            height: bb.height.round(),
        });

        let liveness = to_liveness(pv_face.liveness.as_ref(), pv_face.liveness_validness.as_ref());

        Self {
            bbox,
            acceptability: Some(pv_face.acceptability),
            quality: Some(pv_face.quality),
            mask: Some(pv_face.mask),
            template: None,
            liveness,
        }
    }
}
fn to_liveness(
    liveness: Option<&processor::Liveness>,
    validness: Option<&processor::Validness>,
) -> Option<Liveness> {
    // 1. Early return if liveness is None.
    let liveness = liveness?;

    let is_valid = validness.map_or(false, |v| v.is_valid);

    // 3. Process feedback without double-evaluating validness
    let feedback = validness
        .map(|v| v.feedback.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|code| {
            processor::validness::Feedback::try_from(*code)
                .unwrap_or(processor::validness::Feedback::Unknown)
                .as_str_name()
                .to_string()
        })
        .collect();

    let is_live = is_valid && liveness.liveness_probability > 0.5;

    // Return the final mapped struct wrapped in Some
    Some(Liveness { is_live, feedback, score: liveness.liveness_probability })
}

pub(crate) fn timestamp_to_rfc3339(timestamp: Option<prost_types::Timestamp>) -> String {
    let Some(timestamp) = timestamp else {
        return String::new();
    };

    let Some(datetime) = DateTime::<Utc>::from_timestamp(timestamp.seconds, timestamp.nanos as u32)
    else {
        return String::new();
    };

    datetime.to_rfc3339_opts(SecondsFormat::Micros, true)
}
