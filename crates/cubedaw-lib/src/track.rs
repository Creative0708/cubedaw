use std::collections::BTreeMap;

use crate::{Id, IdMap, IdSet, Patch, Range, Section};

#[derive(Debug, Clone)]
pub struct Track {
    pub patch: Patch,
    pub inner: TrackInner,
}

impl Track {
    pub fn new_section(patch: Patch) -> Self {
        Self {
            patch,
            inner: TrackInner::Section(SectionTrack::new()),
        }
    }

    pub fn new_group(patch: Patch) -> Self {
        Self {
            patch,
            inner: TrackInner::Group(GroupTrack::new()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TrackInner {
    Section(SectionTrack),
    Group(GroupTrack),
}

impl TrackInner {
    pub fn section(&self) -> Option<&SectionTrack> {
        match self {
            Self::Section(synth_track) => Some(synth_track),
            _ => None,
        }
    }
    pub fn section_mut(&mut self) -> Option<&mut SectionTrack> {
        match self {
            Self::Section(synth_track) => Some(synth_track),
            _ => None,
        }
    }
    pub fn group(&self) -> Option<&GroupTrack> {
        match self {
            Self::Group(group_track) => Some(group_track),
            _ => None,
        }
    }
    pub fn group_mut(&mut self) -> Option<&mut GroupTrack> {
        match self {
            Self::Group(group_track) => Some(group_track),
            _ => None,
        }
    }

    pub fn is_section(&self) -> bool {
        self.section().is_some()
    }
    pub fn is_group(&self) -> bool {
        self.group().is_some()
    }
}

#[derive(Debug, Clone)]
pub struct SectionTrack {
    polyphony: u32,
    section_map: IdMap<Section>,
    sections: BTreeMap<Range, Id<Section>>,
}

impl SectionTrack {
    pub fn new() -> Self {
        Self {
            polyphony: 32,
            section_map: Default::default(),
            sections: Default::default(),
        }
    }

    pub fn check_overlap(&self) {
        // TODO change to map_windows when it's stabilized

        let mut prev_range = None;
        for &range in self.sections.keys() {
            if let Some(prev_range) = prev_range {
                if range.intersects(prev_range) {
                    panic!(
                        "Section of range {range:?} would overlap with other section of range {prev_range:?}"
                    );
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

    pub fn add_section(
        &mut self,
        section_id: Id<Section>,
        start_pos: i64,
        section: Section,
    ) -> &mut Section {
        let section_range = Range::from_start_length(start_pos, section.length);
        self.check_overlap_with(section_range);

        let section = self.section_map.insert_and_get_mut(section_id, section);
        self.sections.insert(section_range, section_id);
        section
    }

    pub fn remove_section(&mut self, section_id: Id<Section>, start_pos: i64) -> Section {
        let section = self.section_map.take(section_id);
        let removed = self
            .sections
            .remove(&Range::from_start_length(start_pos, section.length));
        assert_eq!(
            removed,
            Some(section_id),
            "section id in track internal map doesn't match removed id"
        );
        section
    }
    pub fn remove_section_from_range(&mut self, section_range: Range) -> (Id<Section>, Section) {
        let section_id = self
            .sections
            .remove(&section_range)
            .expect("section range does not exist");
        let section = self.section_map.take(section_id);
        (section_id, section)
    }

    pub fn move_section(&mut self, section_range: Range, new_start_pos: i64) {
        let Some(section_id) = self.sections.remove(&section_range) else {
            panic!("Track::move_section was given nonexistent Section: {section_range:?}");
        };

        let new_range = section_range + (new_start_pos - section_range.start);
        self.check_overlap_with(new_range);

        self.sections.insert(new_range, section_id);
    }

    pub fn section_at(&self, pos: i64) -> Option<(Range, Id<Section>)> {
        self.sections
            .range(..Range::unbounded_end(pos))
            .next_back()
            .and_then(|(&range, &id)| range.contains(pos).then_some((range, id)))
    }

    pub fn sections_intersecting(
        &self,
        range: Range,
    ) -> impl '_ + Iterator<Item = (Range, Id<Section>)> {
        // there doesn't seem to be a way to step before the start of an iterator like in C++. so we just bite the extra range call
        let possible_first_section = self.section_at(range.start);
        let other_sections = self
            .sections
            .range(Range::unbounded_end(range.start)..Range::unbounded_end(range.end))
            .map(|(&range, &id)| (range, id));
        possible_first_section.into_iter().chain(other_sections)
    }

    pub fn section(&self, id: Id<Section>) -> Option<&Section> {
        self.section_map.get(id)
    }
    pub fn section_mut(&mut self, id: Id<Section>) -> Option<&mut Section> {
        self.section_map.get_mut(id)
    }

    pub fn sections(&self) -> impl Iterator<Item = (Range, Id<Section>, &Section)> {
        self.sections.iter().map(|(&range, &id)| {
            (
                range,
                id,
                self.section_map.get(id).unwrap_or_else(|| unreachable!()),
            )
        })
    }

    pub fn polyphony(&self) -> u32 {
        self.polyphony
    }
    pub fn set_polyphony(&mut self, polyphony: u32) {
        self.polyphony = polyphony;
    }
}
impl Default for SectionTrack {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct GroupTrack {
    pub children: IdSet<Track>,
}

impl GroupTrack {
    pub fn new() -> Self {
        Default::default()
    }
}
