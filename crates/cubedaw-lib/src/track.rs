use std::collections::BTreeMap;

use crate::{Id, IdMap, Range, Section};

#[derive(Debug)]
pub struct Track {
    sections: BTreeMap<Range, Id<Section>>,
}

impl Track {
    pub fn new() -> Self {
        Self {
            sections: BTreeMap::new(),
        }
    }

    pub fn check_overlap(&self) {
        for [r1, r2] in self.sections.keys().map_windows(|&[&r1, &r2]| [r1, r2]) {
            if r1.intersects(r2) {
                panic!("Section of range {r1:?} would overlap with other section of range {r2:?}");
            }
        }
    }

    pub fn check_overlap_with(&self, range: Range) {
        // mmm function chaining
        if let Some(overlapping_range) = self
            .sections
            .range(range..)
            .next()
            .and_then(|(&other_range, _)| (other_range.start < range.end).then_some(other_range))
            .or_else(|| {
                self.sections
                    .range(..range)
                    .next_back()
                    .and_then(|(&other_range, _)| {
                        (other_range.end > range.start).then_some(other_range)
                    })
            })
        {
            panic!(
                "Range {range:?} would overlap with other section of range {overlapping_range:?}"
            );
        }
    }

    pub fn add_section<'a>(
        &mut self,
        sections: &'a mut IdMap<Section>,
        section: Section,
    ) -> &'a mut Section {
        self.check_overlap_with(section.range);

        let range = section.range;
        let id = sections.create(section);
        self.sections.insert(range, id);
        sections.get_mut(id)
    }

    pub fn move_section(
        &mut self,
        sections: &mut IdMap<Section>,
        section_range: Range,
        new_range: Range,
    ) {
        let Some(section_id) = self.sections.remove(&section_range) else {
            panic!("Track::move_section was given nonexistent Section: {section_range:?}");
        };

        let section = sections.get_mut(section_id);
        section.range = new_range;
        self.sections.insert(new_range, section_id);
    }

    pub fn get_section_at(&mut self, pos: i64) -> Option<Id<Section>> {
        self.sections
            .range(..Range::unbounded_end(pos))
            .next_back()
            .and_then(|(&range, &id)| range.contains(pos).then_some(id))
    }

    pub fn sections(&self) -> impl Iterator<Item = (Range, Id<Section>)> + '_ {
        self.sections.iter().map(|x| (*x.0, *x.1))
    }
}
