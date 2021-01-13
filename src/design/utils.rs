use super::data::*;
/// The return type for methods that ask if a nucleotide is the end of a domain/strand/xover
#[derive(Debug, Clone, Copy)]
pub enum Extremity {
    No,
    Prime3,
    Prime5,
}

impl Extremity {
    pub fn is_3prime(&self) -> bool {
        match self {
            Extremity::Prime3 => true,
            _ => false,
        }
    }

    pub fn is_5prime(&self) -> bool {
        match self {
            Extremity::Prime5 => true,
            _ => false,
        }
    }

    pub fn is_end(&self) -> bool {
        match self {
            Extremity::No => false,
            _ => true,
        }
    }

    pub fn to_opt(&self) -> Option<bool> {
        match self {
            Extremity::No => None,
            Extremity::Prime3 => Some(true),
            Extremity::Prime5 => Some(false),
        }
    }
}

/// This structure contains the information required to know how to make a cross-over between two
/// nucleotides.
#[derive(Debug)]
pub struct XoverInfo {
    /// The source strand data
    pub source: Strand,
    /// The target strand data
    pub target: Strand,
    /// The id of the source strand
    pub source_id: usize,
    /// The id of the target strand
    pub target_id: usize,
    /// The source nucleotide
    pub source_nucl: Nucl,
    /// The target nucleotide
    pub target_nucl: Nucl,
    /// Identifier of the design on which to do the cross-over
    pub design_id: usize,
    /// The target nucl Strand extremity status
    pub target_strand_end: Extremity,
    /// The source nucl Strand extremity status
    pub source_strand_end: Extremity,
}
