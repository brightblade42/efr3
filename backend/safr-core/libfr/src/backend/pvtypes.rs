// ------- This file is for implementing FROM traits
// From Traits allow us do type conversions and keep the main code clean.

use crate::EnrollData;
use crate::Face;
use base64::{engine::general_purpose, Engine as _};

use libpv::types::ProcessFullImageRequest;

//cool
impl From<EnrollData> for ProcessFullImageRequest {
    fn from(data: EnrollData) -> Self {
        ProcessFullImageRequest {
            image: data
                .image
                .map(|bytes| general_purpose::STANDARD.encode(bytes))
                .unwrap_or_default(),
            outputs: Some(vec![
                "EMBEDDING".to_string(),
                "QUALITY".to_string(),
                "MASK".to_string(),
            ]),
            find_most_prominent_face: true,
        }
    }
}

impl From<&EnrollData> for ProcessFullImageRequest {
    fn from(data: &EnrollData) -> Self {
        ProcessFullImageRequest {
            image: data
                .image
                .as_ref()
                .map(|bytes| general_purpose::STANDARD.encode(bytes))
                .unwrap_or_default(),
            outputs: Some(vec![
                "EMBEDDING".to_string(),
                "QUALITY".to_string(),
                "MASK".to_string(),
            ]),
            find_most_prominent_face: true,
        }
    }
}

impl From<libpv::types::Face> for Face {
    fn from(pv_face: libpv::types::Face) -> Self {
        let bbox = match pv_face.bounding_box {
            Some(bb) => Some(crate::BoundingBox {
                origin: crate::Point {
                    x: bb.origin.x.floor(),
                    y: bb.origin.y.floor(),
                },
                width: bb.width.round(),
                height: bb.height.round(),
            }),
            None => None,
        };

        let liveness = to_liveness(
            pv_face.liveness.as_ref(),
            pv_face.liveness_validness.as_ref(),
        );

        Self {
            bbox,
            acceptability: pv_face.acceptability,
            quality: pv_face.quality,
            mask: pv_face.mask,
            liveness,
        }
    }
}

impl From<&libpv::types::Face> for Face {
    fn from(pv_face: &libpv::types::Face) -> Self {
        let bbox = pv_face.bounding_box.as_ref().map(|bb| crate::BoundingBox {
            origin: crate::Point {
                x: bb.origin.x.floor(),
                y: bb.origin.y.floor(),
            },
            width: bb.width.round(),
            height: bb.height.round(),
        });

        let liveness = to_liveness(
            pv_face.liveness.as_ref(),
            pv_face.liveness_validness.as_ref(),
        );

        Self {
            bbox,
            acceptability: pv_face.acceptability,
            quality: pv_face.quality,
            mask: pv_face.mask,
            liveness,
        }
    }
}

fn to_liveness(
    liveness: Option<&libpv::types::Liveness>,
    validness: Option<&libpv::types::Validness>,
) -> Option<crate::Liveness> {
    liveness.map(|liveness| {
        let feedback = validness
            .map(|item| item.feedback.clone())
            .unwrap_or_default();
        let is_live = validness.map(|item| item.is_valid).unwrap_or(false)
            && liveness.liveness_probability > 0.5;

        crate::Liveness {
            is_live,
            feedback,
            score: liveness.liveness_probability,
        }
    })
}
