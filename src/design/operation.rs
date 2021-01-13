//! This module defines methods for the `Design` type that are meant to break high level operation
//! on a design into atomic undoable operations

use super::*;
use crate::mediator::{CrossCut, Cut, Operation, Xover}; // TODO, those should maybe be defined in this module

impl Design {
    /// Make a cross-over between source and target.
    ///
    /// If source and target are both the 5' end of a strand, or if they are both the 3' end of a
    /// strand, a cross-over between them is considered impossible and this methods returns an
    /// empty vector of operations.
    ///
    /// Otherwise, this methods return a vector of operations that will create a cross-over between
    /// source and target.
    /// When possible, source will be the 5' end of the cross-over and target will be the 3' end.
    /// This is not possible when source is the 5' end of a strand or when target is the 3' end of
    /// a strand. In these cases, source will be the 3' end of the cross-over and target will be
    /// the 5'end of the cross-over.
    pub fn general_cross_over(&self, source: Nucl, target: Nucl) -> Vec<Arc<dyn Operation>> {
        let xover = self.get_xover_info(source, target);
        if let Some(xover) = xover {
            match (
                xover.source_strand_end.to_opt(),
                xover.target_strand_end.to_opt(),
            ) {
                (Some(true), Some(true)) | (Some(false), Some(false)) => vec![], // xover can't be done,
                (Some(true), Some(false)) => {
                    // We can xover directly
                    vec![Arc::new(Xover {
                        strand_5prime: xover.source,
                        strand_3prime: xover.target,
                        prime5_id: xover.source_id,
                        prime3_id: xover.target_id,
                        undo: false,
                        design_id: xover.design_id,
                    })]
                }
                (Some(false), Some(true)) => {
                    // We can xover directly but we must reverse the xover
                    vec![Arc::new(Xover {
                        strand_5prime: xover.target,
                        strand_3prime: xover.source,
                        prime5_id: xover.target_id,
                        prime3_id: xover.source_id,
                        undo: false,
                        design_id: xover.design_id,
                    })]
                }
                (Some(b), None) => {
                    // We can cut cross directly
                    let target_3prime = b;
                    vec![Arc::new(CrossCut {
                        target_3prime,
                        target_strand: xover.target,
                        source_strand: xover.source,
                        nucl: xover.target_nucl,
                        design_id: xover.design_id,
                        target_id: xover.target_id,
                        source_id: xover.source_id,
                        undo: false,
                    })]
                }
                (None, Some(b)) => {
                    // We can cut cross directly but we need to reverse the xover
                    let target_3prime = b;
                    vec![Arc::new(CrossCut {
                        target_3prime,
                        target_strand: xover.source,
                        source_strand: xover.target,
                        nucl: xover.source_nucl,
                        design_id: xover.design_id,
                        target_id: xover.source_id,
                        source_id: xover.target_id,
                        undo: false,
                    })]
                }
                (None, None) => {
                    let mut ret: Vec<Arc<dyn Operation>> = Vec::new();
                    // We must cut the source strand first
                    ret.push(Arc::new(Cut {
                        nucl: xover.source_nucl,
                        strand_id: xover.source_id,
                        strand: xover.source,
                        undo: false,
                        design_id: xover.design_id,
                    }));
                    // And we must get back the resulting strand
                    let source_strand = self.get_raw_strand(xover.source_id).unwrap();
                    ret.push(Arc::new(CrossCut {
                        target_3prime: true,
                        target_strand: xover.target,
                        source_strand,
                        nucl: xover.target_nucl,
                        design_id: xover.design_id,
                        target_id: xover.target_id,
                        source_id: xover.source_id,
                        undo: false,
                    }));
                    ret
                }
            }
        } else {
            vec![]
        }
    }
}
