use std::collections::BTreeMap;

use ahash::HashSetExt;

use crate::{Clip, Id, IdMap, IdSet, Patch, Range};

#[derive(Debug, Clone)]
pub struct Track {
    pub patch: Patch,

    polyphony: u32,

    // these two fields are kept synchronized with one another
    clip_map: IdMap<Clip>,
    clips: BTreeMap<Range, Id<Clip>>,

    pub children: IdSet<Track>,
}

impl Track {
    pub fn new(patch: Patch) -> Self {
        Self {
            patch,
            polyphony: 32,

            clip_map: Default::default(),
            clips: Default::default(),

            children: IdSet::new(),
        }
    }

    pub fn check_overlap(&self) {
        // TODO change to map_windows when it's stabilized

        let mut prev_range = None;
        for &range in self.clips.keys() {
            if let Some(prev_range) = prev_range {
                if range.intersects(prev_range) {
                    panic!(
                        "Clip of range {range:?} would overlap with other clip of range {prev_range:?}"
                    );
                }
            }
            prev_range = Some(range);
        }
    }

    pub fn check_overlap_with(&self, range: Range) {
        // mmm function chaining
        if let Some(overlapping_range) = self
            .clips
            .range(range..)
            .next()
            .and_then(|(&other_range, _)| (other_range.start < range.end).then_some(other_range))
            .or_else(|| {
                self.clips
                    .range(..range)
                    .next_back()
                    .and_then(|(&other_range, _)| {
                        (other_range.end > range.start).then_some(other_range)
                    })
            })
        {
            panic!("Range {range:?} would overlap with other clip of range {overlapping_range:?}");
        }
    }

    pub fn add_clip(&mut self, clip_id: Id<Clip>, start_pos: i64, clip: Clip) -> &mut Clip {
        let clip_range = Range::from_start_length(start_pos, clip.length);
        self.check_overlap_with(clip_range);

        let clip = self.clip_map.insert_and_get_mut(clip_id, clip);
        self.clips.insert(clip_range, clip_id);
        clip
    }

    pub fn remove_clip(&mut self, clip_id: Id<Clip>, start_pos: i64) -> Clip {
        let clip = self.clip_map.take(clip_id);
        let removed = self
            .clips
            .remove(&Range::from_start_length(start_pos, clip.length));
        assert_eq!(
            removed,
            Some(clip_id),
            "clip id in track internal map doesn't match removed id"
        );
        clip
    }
    pub fn remove_clip_from_range(&mut self, clip_range: Range) -> (Id<Clip>, Clip) {
        let clip_id = self
            .clips
            .remove(&clip_range)
            .expect("clip range does not exist");
        let clip = self.clip_map.take(clip_id);
        (clip_id, clip)
    }

    pub fn move_clip(&mut self, clip_range: Range, new_start_pos: i64) {
        let Some(clip_id) = self.clips.remove(&clip_range) else {
            panic!("Track::move_clip was given nonexistent Clip: {clip_range:?}");
        };

        let new_range = clip_range + (new_start_pos - clip_range.start);
        self.check_overlap_with(new_range);

        self.clips.insert(new_range, clip_id);
    }

    pub fn clip_at(&self, pos: i64) -> Option<(Range, Id<Clip>)> {
        self.clips
            .range(..Range::unbounded_end(pos))
            .next_back()
            .and_then(|(&range, &id)| range.contains(pos).then_some((range, id)))
    }

    pub fn clips_intersecting(&self, range: Range) -> impl '_ + Iterator<Item = (Range, Id<Clip>)> {
        // there doesn't seem to be a way to step before the start of an iterator like in C++. so we just bite the extra range call
        let possible_first_clip = self.clip_at(range.start);
        let other_clips = self
            .clips
            .range(Range::unbounded_end(range.start)..Range::unbounded_end(range.end))
            .map(|(&range, &id)| (range, id));
        possible_first_clip.into_iter().chain(other_clips)
    }

    pub fn clip(&self, id: Id<Clip>) -> Option<&Clip> {
        self.clip_map.get(id)
    }
    pub fn clip_mut(&mut self, id: Id<Clip>) -> Option<&mut Clip> {
        self.clip_map.get_mut(id)
    }

    pub fn clips(&self) -> impl Iterator<Item = (Range, Id<Clip>, &Clip)> {
        self.clips.iter().map(|(&range, &id)| {
            (
                range,
                id,
                self.clip_map.get(id).unwrap_or_else(|| unreachable!()),
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
