use std::collections::{btree_map::Entry, BTreeMap};

use crate::{id::IdCorrespondenceMap as _, synth::Synthesizer, Id, IdMap, Range};

mod note;
mod section;

pub use note::*;
pub use section::*;

#[derive(Clone, Debug)]
pub struct Track {
    pub name: String,
    pub id: Id<Track>,
    pub volume: f32,

    pub track_data: TrackData,
}

impl Track {
    pub fn dbg_new(name: String) -> Self {
        let id = Id::new(name.as_str());
        Self {
            name,
            id,
            volume: 0.5,
            track_data: TrackData::SynthesizerTrack(SynthesizerTrackData {
                synth: Synthesizer::dbg_new(),
                sections: BTreeMap::new(),
            }),
        }
    }
}

impl PartialEq for Track {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Track {}

#[derive(Clone, Debug)]
pub enum TrackData {
    ParentTrack(Vec<Id<Track>>),
    SynthesizerTrack(SynthesizerTrackData),
}

#[derive(Clone, Debug)]
pub struct SynthesizerTrackData {
    synth: Synthesizer,
    /// A set of sections, ordered by starting position. No two sections should ever overlap.
    pub sections: BTreeMap<i64, Id<Section>>,
}

impl SynthesizerTrackData {
    pub fn create_section<'a>(
        &mut self,
        section_map: &'a mut IdMap<Section, Section>,
        range: Range,
    ) -> Option<&'a mut Section> {
        log::info!("creating section! {:?}", range);

        let start_pos = range.start.max(
            self.sections
                .range(..range.start)
                .next_back()
                .map_or(i64::MIN, |(_, &id)| section_map.id_get(id).end()),
        );
        let end_pos = range.end.min(
            self.sections
                .range(range.start..)
                .next()
                .map_or(i64::MAX, |(&start_pos, _)| start_pos),
        );

        let target_range = Range::new(start_pos, end_pos);
        if target_range.valid() {
            let Entry::Vacant(entry) = self.sections.entry(start_pos) else {
                unreachable!()
            };

            let section_id = Id::arbitrary();
            log::info!(
                "created section with id {:?}, {:?}",
                section_id,
                Id::<()>::new(2)
            );

            let section = Section::empty(target_range);

            entry.insert(section_id);

            Some(section_map.id_set(section_id, section))
        } else {
            None
        }
    }

    pub fn get_or_create_section_at<'a>(
        &mut self,
        section_map: &'a mut IdMap<Section, Section>,
        pos: i64,
    ) -> &'a mut Section {
        let res = self
            .sections
            .range(..=pos)
            .next_back()
            .and_then(|(_, &id)| {
                // TODO we're getting the section twice due to limitations of the borrow checker
                // so when polonius drops update this function to not do that
                let section: &mut Section = section_map.id_get_mut(id);
                log::info!("{:?} {} {}", section, pos, pos < section.end());
                (pos < section.end()).then_some(id)
            });

        log::info!("{} {:?}", pos, self.sections);

        if let Some(section_id) = res {
            section_map.id_get_mut(section_id)
        } else {
            self.create_section(section_map, Range::surrounding_pos(pos))
                .expect("unimplemented")
        }
    }

    pub fn section_ids(&mut self) -> impl Iterator<Item = Id<Section>> + '_ {
        self.sections.iter().map(|(_, id)| *id)
    }
}
