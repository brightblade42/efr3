use crate::Face;
use libpv::types::{
    AddFaceInput, CreateIdentitiesInput, Embedding, LookupInput, ProcessImageResponse,
};

pub(crate) fn create_identities_input_from_processed(
    processed: &ProcessImageResponse,
    threshold: f32,
) -> CreateIdentitiesInput {
    let face_idx = match processed.most_prominent_face_idx {
        Some(-1) => 0_usize,
        Some(index) => index as usize,
        None => 0_usize,
    };

    let (embedding, quality) = processed
        .faces
        .as_ref()
        .and_then(|faces| faces.get(face_idx))
        .map(|face| {
            (
                face.embedding.clone().unwrap_or_default(),
                face.quality.unwrap_or(0.0),
            )
        })
        .unwrap_or_else(|| (vec![], 0.0));

    CreateIdentitiesInput {
        embeddings: vec![Embedding { embedding }],
        threshold,
        qualities: vec![quality],
        group_ids: None,
        external_ids: None,
    }
}

pub(crate) fn lookup_input_from_processed(
    processed: ProcessImageResponse,
    limit: i32,
) -> LookupInput {
    let Some(face_list) = processed.faces else {
        return LookupInput {
            embeddings: vec![],
            faces: None,
            limit,
        };
    };

    let mut embeddings = Vec::new();
    let mut faces = Vec::new();

    for face in face_list {
        if let Some(embedding) = face.embedding.clone() {
            embeddings.push(Embedding { embedding });
            faces.push(face);
        }
    }

    LookupInput {
        embeddings,
        faces: Some(faces),
        limit,
    }
}

pub(crate) fn add_face_input_from_processed(
    processed: ProcessImageResponse,
    identity_id: String,
    threshold: f32,
) -> AddFaceInput {
    let Some(faces) = processed.faces else {
        return AddFaceInput {
            identity_id,
            embeddings: vec![],
            threshold,
            qualities: vec![],
        };
    };

    let mut embeddings = Vec::new();
    let mut qualities = Vec::new();

    for face in faces {
        if let Some(embedding) = face.embedding {
            embeddings.push(Embedding { embedding });
            if let Some(quality) = face.quality {
                qualities.push(quality);
            }
        }
    }

    AddFaceInput {
        identity_id,
        embeddings,
        threshold,
        qualities,
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
