// ------- This file is for implementing FROM traits
// From Traits allow us do type conversions and keep the main code clean.

use crate::EnrollData;
use crate::Face;

use libpv::types::ProcessFullImageRequest;

//cool
impl From<EnrollData> for ProcessFullImageRequest {
    fn from(data: EnrollData) -> Self {
        ProcessFullImageRequest {
            image: data.image.unwrap_or_default(),
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
            image: data.image.as_ref().cloned().unwrap_or_default(),
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

        Self {
            bbox,
            quality: pv_face.quality,
            mask: pv_face.mask,
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

        Self {
            bbox,
            quality: pv_face.quality,
            mask: pv_face.mask,
        }
    }
}
