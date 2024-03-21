use std::collections::BTreeMap;

use crate::{Id, IdMap, Section};

#[derive(Debug)]
pub struct Track {
    // BTreeMap<starting position, section id>
    sections: BTreeMap<i64, Id<Section>>,
}

impl Track {
    pub fn new() -> Self {
        Self {
            sections: BTreeMap::new(),
        }
    }

    pub fn add_section(&mut self, sections: &mut IdMap<Section>, section: Section) {
        if self
            .sections
            .range(section.start()..)
            .next_back()
            .map_or(false, |(&pos, _)| pos >= section.end())
        {
            panic!(
                "Section {:?} would overlap with other section {:?}",
                section,
                sections.get(
                    *self
                        .sections
                        .range(section.start()..)
                        .next_back()
                        .unwrap()
                        .1
                )
            )
        }
    }
}
