//! Locate and parse the JSON metadata embedded in an ISF fragment shader.
//!
//! ISF metadata normally appears in a block comment beginning with `/*{`.
//! Ordinary comments may precede that block, so detection scans all block
//! comments rather than assuming that metadata begins at byte zero.

use crate::isf_types::{
    IsfDocument,
    IsfMetadata,
};


pub fn looks_like_isf(
    source: &str,
) -> bool {

    parse(
        source
    )
    .is_ok()
}


pub fn parse(
    source: &str,
) -> Result<IsfDocument, String> {

    let mut search_offset =
        0_usize;


    while let Some(relative_start) =
        source[
            search_offset..
        ]
        .find(
            "/*"
        )
    {
        let comment_start =
            search_offset
                + relative_start;


        let content_start =
            comment_start
                + 2;


        let Some(relative_end) =
            source[
                content_start..
            ]
            .find(
                "*/"
            )
        else {
            return Err(
                "Unterminated block comment while searching for ISF metadata"
                    .to_string()
            );
        };


        let comment_end =
            content_start
                + relative_end;


        let comment_body =
            source[
                content_start
                    ..comment_end
            ]
            .trim();


        if comment_body.starts_with(
            '{'
        ) {
            if let Ok(metadata) =
                serde_json::from_str::<IsfMetadata>(
                    comment_body
                )
            {
                if metadata_looks_like_isf(
                    &metadata,
                    comment_body,
                ) {
                    let metadata_end =
                        comment_end
                            + 2;


                    let mut shader_source =
                        String::with_capacity(
                            source.len()
                        );


                    shader_source.push_str(
                        &source[
                            ..comment_start
                        ]
                    );


                    shader_source.push_str(
                        &source[
                            metadata_end..
                        ]
                    );


                    return Ok(
                        IsfDocument {
                            metadata,
                            metadata_start:
                                comment_start,
                            metadata_end,
                            shader_source,
                        }
                    );
                }
            }
        }


        search_offset =
            comment_end
                + 2;
    }


    Err(
        "No valid ISF JSON metadata block was found"
            .to_string()
    )
}


fn metadata_looks_like_isf(
    metadata: &IsfMetadata,
    json_source: &str,
) -> bool {

    metadata.version.is_some()
        || !metadata.inputs.is_empty()
        || !metadata.categories.is_empty()
        || !metadata.passes.is_empty()
        || json_source.contains(
            "\"INPUTS\""
        )
        || json_source.contains(
            "\"ISFVSN\""
        )
        || json_source.contains(
            "\"PASSES\""
        )
        || json_source.contains(
            "\"CATEGORIES\""
        )
}

