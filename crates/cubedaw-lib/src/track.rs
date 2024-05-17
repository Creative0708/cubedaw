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
        // TODO change to map_windows when it's stabilized

        let mut prev_range = None;
        for &range in self.sections.keys() {
            if let Some(prev_range) = prev_range {
                if range.intersects(prev_range) {
                    panic!("Section of range {range:?} would overlap with other section of range {prev_range:?}");
                }
            }
            prev_range = Some(range);
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
        section_id: Id<Section>,
        start_pos: i64,
        section: Section,
    ) -> &'a mut Section {
        let section_range = Range::start_length(start_pos, section.length);
        self.check_overlap_with(section_range);

        let section = sections.set_and_get_mut(section_id, section);
        self.sections.insert(section_range, section_id);
        section
    }

    pub fn remove_section(
        &mut self,
        sections: &mut IdMap<Section>,
        section_id: Id<Section>,
        start_pos: i64,
    ) -> Section {
        let section = sections
            .remove(section_id)
            .expect("tried to delete nonexistent section");
        assert!(
            self.sections
                .remove(&Range::start_length(start_pos, section.length))
                == Some(section_id),
            "section id in track internal map doesn't match removed id"
        );
        section
    }

    pub fn move_section(&mut self, section_range: Range, new_start_pos: i64) {
        let Some(section_id) = self.sections.remove(&section_range) else {
            panic!("Track::move_section was given nonexistent Section: {section_range:?}");
        };

        let new_range = section_range + (new_start_pos - section_range.start);
        self.check_overlap_with(new_range);

        self.sections.insert(new_range, section_id);
    }

    pub fn get_section_at(&self, pos: i64) -> Option<(Range, Id<Section>)> {
        self.sections
            .range(..Range::unbounded_end(pos))
            .next_back()
            .and_then(|(&range, &id)| range.contains(pos).then_some((range, id)))
    }

    pub fn sections(&self) -> impl Iterator<Item = (Range, Id<Section>)> + '_ {
        self.sections.iter().map(|x| (*x.0, *x.1))
    }
}
