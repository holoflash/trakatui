pub mod channel;
pub mod file;
pub mod pattern;
pub mod sample;

pub use channel::Track;
pub use pattern::{Cell, Note, Pattern, PatternColor};
pub use sample::SampleData;

use crate::app::scale::ScaleIndex;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum ArrangerItem {
    Single {
        pattern_idx: usize,
    },
    Group {
        name: String,
        color: Option<PatternColor>,
        repeat: u16,
        pattern_indices: Vec<usize>,
        clone_id: Option<u64>,
        collapsed: bool,
    },
}

impl ArrangerItem {
    pub fn pattern_indices(&self) -> Vec<usize> {
        match self {
            Self::Single { pattern_idx } => vec![*pattern_idx],
            Self::Group { pattern_indices, .. } => pattern_indices.clone(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Project {
    pub patterns: Vec<Pattern>,
    pub arranger: Vec<ArrangerItem>,
    pub current_item_idx: usize,
    pub current_sub_pattern_idx: usize,
    pub tracks: Vec<Track>,
    pub step: usize,
    pub scale_index: ScaleIndex,
    pub transpose: i8,
    pub master_volume_db: f32,
    pub next_clone_id: u64,
}

impl Project {
    pub fn new() -> Self {
        Self {
            patterns: vec![Pattern::new("Pattern 01".into(), 1, 16)],
            arranger: vec![ArrangerItem::Single { pattern_idx: 0 }],
            current_item_idx: 0,
            current_sub_pattern_idx: 0,
            tracks: Track::defaults(),
            step: 1,
            scale_index: ScaleIndex::default(),
            transpose: 0,
            master_volume_db: 0.0,
            next_clone_id: 1,
        }
    }

    pub fn current_pattern_idx(&self) -> usize {
        match &self.arranger[self.current_item_idx] {
            ArrangerItem::Single { pattern_idx } => *pattern_idx,
            ArrangerItem::Group { pattern_indices, .. } => {
                let sub = self.current_sub_pattern_idx.min(pattern_indices.len().saturating_sub(1));
                pattern_indices[sub]
            }
        }
    }

    pub fn current_pattern(&self) -> &Pattern {
        &self.patterns[self.current_pattern_idx()]
    }

    pub fn current_pattern_mut(&mut self) -> &mut Pattern {
        let idx = self.current_pattern_idx();
        &mut self.patterns[idx]
    }

    pub fn flat_order(&self) -> Vec<usize> {
        let mut order = Vec::new();
        for item in &self.arranger {
            match item {
                ArrangerItem::Single { pattern_idx } => {
                    let repeat = self.patterns[*pattern_idx].repeat.max(1) as usize;
                    for _ in 0..repeat {
                        order.push(*pattern_idx);
                    }
                }
                ArrangerItem::Group {
                    repeat,
                    pattern_indices,
                    ..
                } => {
                    let group_repeat = (*repeat).max(1) as usize;
                    for _ in 0..group_repeat {
                        for &pat_idx in pattern_indices {
                            let pat_repeat = self.patterns[pat_idx].repeat.max(1) as usize;
                            for _ in 0..pat_repeat {
                                order.push(pat_idx);
                            }
                        }
                    }
                }
            }
        }
        order
    }

    pub fn flat_order_to_item_idx(&self, flat_idx: usize) -> (usize, usize) {
        let mut pos = 0usize;
        for (item_idx, item) in self.arranger.iter().enumerate() {
            match item {
                ArrangerItem::Single { pattern_idx } => {
                    let slots = self.patterns[*pattern_idx].repeat.max(1) as usize;
                    if flat_idx < pos + slots {
                        return (item_idx, 0);
                    }
                    pos += slots;
                }
                ArrangerItem::Group { repeat, pattern_indices, .. } => {
                    let group_repeat = (*repeat).max(1) as usize;
                    let inner: usize = pattern_indices
                        .iter()
                        .map(|&pi| self.patterns[pi].repeat.max(1) as usize)
                        .sum();
                    let total = group_repeat * inner;
                    if flat_idx < pos + total {
                        let offset = (flat_idx - pos) % inner;
                        let mut sub_pos = 0usize;
                        for (sub_idx, &pi) in pattern_indices.iter().enumerate() {
                            let pat_slots = self.patterns[pi].repeat.max(1) as usize;
                            if offset < sub_pos + pat_slots {
                                return (item_idx, sub_idx);
                            }
                            sub_pos += pat_slots;
                        }
                        return (item_idx, 0);
                    }
                    pos += total;
                }
            }
        }
        (self.arranger.len().saturating_sub(1), 0)
    }

    pub fn item_idx_to_flat_start(&self, target_idx: usize) -> usize {
        let mut pos = 0usize;
        for (item_idx, item) in self.arranger.iter().enumerate() {
            if item_idx == target_idx {
                return pos;
            }
            match item {
                ArrangerItem::Single { pattern_idx } => {
                    pos += self.patterns[*pattern_idx].repeat.max(1) as usize;
                }
                ArrangerItem::Group { repeat, pattern_indices, .. } => {
                    let group_repeat = (*repeat).max(1) as usize;
                    let inner: usize = pattern_indices
                        .iter()
                        .map(|&pi| self.patterns[pi].repeat.max(1) as usize)
                        .sum();
                    pos += group_repeat * inner;
                }
            }
        }
        pos
    }

    pub fn master_volume_linear(&self) -> f32 {
        if self.master_volume_db <= -60.0 {
            0.0
        } else {
            10.0_f32.powf(self.master_volume_db / 20.0)
        }
    }

    pub fn add_track(&mut self) {
        let idx = self.tracks.len();
        self.tracks
            .push(Track::new_empty(&format!("Track {:02}", idx)));
        let pat_idx = self.current_pattern_idx();
        self.patterns[pat_idx].add_channel();
    }

    pub fn delete_track(&mut self, idx: usize) {
        if self.tracks.len() <= 1 || idx >= self.tracks.len() {
            return;
        }
        self.tracks.remove(idx);
        let pat_idx = self.current_pattern_idx();
        self.patterns[pat_idx].remove_channel(idx);
    }

    pub fn next_pattern_name(&self) -> String {
        format!("Pattern {:02}", self.patterns.len() + 1)
    }

    pub fn increment_name(name: &str) -> String {
        let trimmed = name.trim_end();
        if let Some(pos) = trimmed.rfind(|c: char| !c.is_ascii_digit()) {
            let prefix = &trimmed[..=pos];
            let num_str = &trimmed[pos + 1..];
            if let Ok(n) = num_str.parse::<u32>() {
                let width = num_str.len();
                return format!("{}{:0>width$}", prefix, n + 1);
            }
        }
        format!("{} 2", trimmed)
    }

    pub fn duplicate_item(&mut self, item_idx: usize) {
        match self.arranger[item_idx].clone() {
            ArrangerItem::Single { pattern_idx } => {
                let source = &self.patterns[pattern_idx];
                let new_name = Self::increment_name(&source.name);
                let mut new_pat = source.clone();
                new_pat.name = new_name;
                let new_idx = self.patterns.len();
                self.patterns.push(new_pat);
                self.arranger
                    .insert(item_idx + 1, ArrangerItem::Single { pattern_idx: new_idx });
            }
            ArrangerItem::Group {
                name,
                color,
                repeat,
                pattern_indices,
                ..
            } => {
                let new_name = Self::increment_name(&name);
                let mut new_indices = Vec::new();
                for &pi in &pattern_indices {
                    let mut new_pat = self.patterns[pi].clone();
                    new_pat.name = Self::increment_name(&new_pat.name);
                    let new_idx = self.patterns.len();
                    self.patterns.push(new_pat);
                    new_indices.push(new_idx);
                }
                self.arranger.insert(
                    item_idx + 1,
                    ArrangerItem::Group {
                        name: new_name,
                        color,
                        repeat,
                        pattern_indices: new_indices,
                        clone_id: None,
                        collapsed: false,
                    },
                );
            }
        }
    }

    pub fn clone_item(&mut self, item_idx: usize) {
        let clone_id = match &mut self.arranger[item_idx] {
            ArrangerItem::Single { pattern_idx } => {
                let new_item = ArrangerItem::Single {
                    pattern_idx: *pattern_idx,
                };
                self.arranger.insert(item_idx + 1, new_item);
                return;
            }
            ArrangerItem::Group { clone_id, .. } => {
                if clone_id.is_none() {
                    *clone_id = Some(self.next_clone_id);
                    self.next_clone_id += 1;
                }
                clone_id.unwrap()
            }
        };
        let cloned = self.arranger[item_idx].clone();
        if let ArrangerItem::Group { .. } = &cloned {
            let mut new_item = cloned;
            if let ArrangerItem::Group {
                clone_id: ref mut cid,
                ..
            } = new_item
            {
                *cid = Some(clone_id);
            }
            self.arranger.insert(item_idx + 1, new_item);
        }
    }

    pub fn delete_item(&mut self, item_idx: usize) {
        if self.arranger.len() <= 1 {
            return;
        }

        let removed = self.arranger.remove(item_idx);
        if self.current_item_idx >= self.arranger.len() {
            self.current_item_idx = self.arranger.len() - 1;
        }

        let removed_indices = removed.pattern_indices();
        let still_used: Vec<usize> = self
            .arranger
            .iter()
            .flat_map(|item| item.pattern_indices())
            .collect();

        let mut to_remove: Vec<usize> = removed_indices
            .into_iter()
            .filter(|idx| !still_used.contains(idx))
            .collect();
        to_remove.sort_unstable();
        to_remove.dedup();
        to_remove.reverse();

        for pat_idx in to_remove {
            self.patterns.remove(pat_idx);
            for item in &mut self.arranger {
                match item {
                    ArrangerItem::Single { pattern_idx } => {
                        if *pattern_idx > pat_idx {
                            *pattern_idx -= 1;
                        }
                    }
                    ArrangerItem::Group {
                        pattern_indices, ..
                    } => {
                        for pi in pattern_indices.iter_mut() {
                            if *pi > pat_idx {
                                *pi -= 1;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn group_items(&mut self, indices: &[usize]) {
        if indices.len() < 2 {
            return;
        }
        let mut sorted = indices.to_vec();
        sorted.sort_unstable();
        sorted.dedup();

        let mut pattern_indices = Vec::new();
        for &i in &sorted {
            if i < self.arranger.len() {
                pattern_indices.extend(self.arranger[i].pattern_indices());
            }
        }

        let insert_pos = sorted[0];
        let group_num = self
            .arranger
            .iter()
            .filter(|item| matches!(item, ArrangerItem::Group { .. }))
            .count()
            + 1;

        for &i in sorted.iter().rev() {
            self.arranger.remove(i);
        }

        let group = ArrangerItem::Group {
            name: format!("Group {}", group_num),
            color: Some(PatternColor::random()),
            repeat: 1,
            pattern_indices,
            clone_id: None,
            collapsed: false,
        };

        let pos = insert_pos.min(self.arranger.len());
        self.arranger.insert(pos, group);
        self.current_item_idx = pos;
    }

    pub fn ungroup(&mut self, item_idx: usize) {
        if let ArrangerItem::Group { pattern_indices, .. } = &self.arranger[item_idx] {
            let indices = pattern_indices.clone();
            self.arranger.remove(item_idx);
            for (offset, pat_idx) in indices.into_iter().enumerate() {
                self.arranger
                    .insert(item_idx + offset, ArrangerItem::Single { pattern_idx: pat_idx });
            }
            if self.current_item_idx >= self.arranger.len() {
                self.current_item_idx = self.arranger.len() - 1;
            }
        }
    }

    pub fn reorder_item(&mut self, from: usize, to: usize) {
        if from == to || from >= self.arranger.len() || to >= self.arranger.len() {
            return;
        }
        let item = self.arranger.remove(from);
        self.arranger.insert(to, item);
        if self.current_item_idx == from {
            self.current_item_idx = to;
        } else if from < self.current_item_idx && to >= self.current_item_idx {
            self.current_item_idx -= 1;
        } else if from > self.current_item_idx && to <= self.current_item_idx {
            self.current_item_idx += 1;
        }
    }

    pub fn reorder_sub_pattern(&mut self, group_idx: usize, from: usize, to: usize) {
        if let ArrangerItem::Group {
            pattern_indices, ..
        } = &mut self.arranger[group_idx]
        {
            if from == to || from >= pattern_indices.len() || to >= pattern_indices.len() {
                return;
            }
            let item = pattern_indices.remove(from);
            pattern_indices.insert(to, item);
        }
    }

    pub fn move_item_into_group(&mut self, item_idx: usize, group_idx: usize, sub_pos: usize) {
        if item_idx >= self.arranger.len() || group_idx >= self.arranger.len() {
            return;
        }
        if !matches!(self.arranger[item_idx], ArrangerItem::Single { .. }) {
            return;
        }
        if !matches!(self.arranger[group_idx], ArrangerItem::Group { .. }) {
            return;
        }
        let ArrangerItem::Single { pattern_idx } = self.arranger[item_idx] else {
            return;
        };

        self.arranger.remove(item_idx);

        let actual_group = if item_idx < group_idx {
            group_idx - 1
        } else {
            group_idx
        };

        if actual_group < self.arranger.len() {
            if let ArrangerItem::Group {
                pattern_indices, ..
            } = &mut self.arranger[actual_group]
            {
                let pos = sub_pos.min(pattern_indices.len());
                pattern_indices.insert(pos, pattern_idx);
            }
        }

        self.current_item_idx = actual_group.min(self.arranger.len().saturating_sub(1));
    }

    pub fn move_sub_pattern_out(
        &mut self,
        group_idx: usize,
        sub_idx: usize,
        target_item_idx: usize,
    ) {
        if group_idx >= self.arranger.len() {
            return;
        }

        let pat_idx = if let ArrangerItem::Group {
            pattern_indices, ..
        } = &mut self.arranger[group_idx]
        {
            if sub_idx >= pattern_indices.len() {
                return;
            }
            pattern_indices.remove(sub_idx)
        } else {
            return;
        };

        let group_now_empty = if let ArrangerItem::Group {
            pattern_indices, ..
        } = &self.arranger[group_idx]
        {
            pattern_indices.is_empty()
        } else {
            false
        };

        if group_now_empty {
            self.arranger.remove(group_idx);
            let adjusted_target = if target_item_idx > group_idx {
                target_item_idx - 1
            } else {
                target_item_idx
            };
            let pos = adjusted_target.min(self.arranger.len());
            self.arranger
                .insert(pos, ArrangerItem::Single { pattern_idx: pat_idx });
            self.current_item_idx = pos;
        } else {
            let pos = target_item_idx.min(self.arranger.len());
            self.arranger
                .insert(pos, ArrangerItem::Single { pattern_idx: pat_idx });
            self.current_item_idx = pos;
        }
    }

    pub fn move_sub_between_groups(
        &mut self,
        from_group: usize,
        from_sub: usize,
        to_group: usize,
        to_sub: usize,
    ) {
        if from_group >= self.arranger.len() || to_group >= self.arranger.len() {
            return;
        }
        if from_group == to_group {
            self.reorder_sub_pattern(from_group, from_sub, to_sub);
            return;
        }

        let pat_idx = if let ArrangerItem::Group {
            pattern_indices, ..
        } = &mut self.arranger[from_group]
        {
            if from_sub >= pattern_indices.len() {
                return;
            }
            pattern_indices.remove(from_sub)
        } else {
            return;
        };

        let group_now_empty = if let ArrangerItem::Group {
            pattern_indices, ..
        } = &self.arranger[from_group]
        {
            pattern_indices.is_empty()
        } else {
            false
        };

        let actual_to = if group_now_empty {
            self.arranger.remove(from_group);
            if from_group < to_group {
                to_group - 1
            } else {
                to_group
            }
        } else {
            to_group
        };

        if actual_to < self.arranger.len() {
            if let ArrangerItem::Group {
                pattern_indices, ..
            } = &mut self.arranger[actual_to]
            {
                let pos = to_sub.min(pattern_indices.len());
                pattern_indices.insert(pos, pat_idx);
            }
        }

        self.current_item_idx = actual_to.min(self.arranger.len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_project() {
        let p = Project::new();
        assert_eq!(p.current_pattern().rows, 16);
        assert_eq!(p.current_pattern().bpm, 120);
        assert_eq!(p.current_pattern().computed_rows(), 16);
        assert_eq!(p.arranger.len(), 1);
        assert_eq!(p.flat_order(), vec![0]);
    }

    #[test]
    fn flat_order_with_repeats() {
        let mut p = Project::new();
        p.patterns[0].repeat = 3;
        assert_eq!(p.flat_order(), vec![0, 0, 0]);
    }

    #[test]
    fn flat_order_with_group() {
        let mut p = Project::new();
        p.patterns.push(Pattern::new("Pattern 02".into(), 1, 16));
        p.arranger = vec![ArrangerItem::Group {
            name: "Group 1".into(),
            color: None,
            repeat: 2,
            pattern_indices: vec![0, 1],
            clone_id: None,
            collapsed: false,
        }];
        assert_eq!(p.flat_order(), vec![0, 1, 0, 1]);
    }

    #[test]
    fn flat_order_group_with_pattern_repeats() {
        let mut p = Project::new();
        p.patterns[0].repeat = 2;
        p.patterns.push(Pattern::new("Pattern 02".into(), 1, 16));
        p.patterns[1].repeat = 3;
        p.arranger = vec![ArrangerItem::Group {
            name: "G1".into(),
            color: None,
            repeat: 1,
            pattern_indices: vec![0, 1],
            clone_id: None,
            collapsed: false,
        }];
        assert_eq!(p.flat_order(), vec![0, 0, 1, 1, 1]);
    }

    #[test]
    fn duplicate_item_single() {
        let mut p = Project::new();
        p.duplicate_item(0);
        assert_eq!(p.patterns.len(), 2);
        assert_eq!(p.arranger.len(), 2);
        assert_eq!(p.patterns[1].name, "Pattern 02");
    }

    #[test]
    fn delete_item() {
        let mut p = Project::new();
        p.patterns.push(Pattern::new("Pattern 02".into(), 1, 16));
        p.arranger.push(ArrangerItem::Single { pattern_idx: 1 });
        p.delete_item(0);
        assert_eq!(p.patterns.len(), 1);
        assert_eq!(p.arranger.len(), 1);
        assert_eq!(p.patterns[0].name, "Pattern 02");
    }

    #[test]
    fn group_and_ungroup() {
        let mut p = Project::new();
        p.patterns.push(Pattern::new("Pattern 02".into(), 1, 16));
        p.arranger.push(ArrangerItem::Single { pattern_idx: 1 });
        p.group_items(&[0, 1]);
        assert_eq!(p.arranger.len(), 1);
        assert!(matches!(&p.arranger[0], ArrangerItem::Group { pattern_indices, .. } if pattern_indices == &[0, 1]));
        p.ungroup(0);
        assert_eq!(p.arranger.len(), 2);
    }

    #[test]
    fn increment_name_with_number() {
        assert_eq!(Project::increment_name("Pattern 01"), "Pattern 02");
        assert_eq!(Project::increment_name("Group 5"), "Group 6");
        assert_eq!(Project::increment_name("Test"), "Test 2");
    }

    #[test]
    fn reorder_item() {
        let mut p = Project::new();
        p.patterns.push(Pattern::new("Pattern 02".into(), 1, 16));
        p.patterns.push(Pattern::new("Pattern 03".into(), 1, 16));
        p.arranger.push(ArrangerItem::Single { pattern_idx: 1 });
        p.arranger.push(ArrangerItem::Single { pattern_idx: 2 });
        p.current_item_idx = 0;
        p.reorder_item(0, 2);
        assert_eq!(p.current_item_idx, 2);
        assert!(matches!(&p.arranger[0], ArrangerItem::Single { pattern_idx: 1 }));
    }

    #[test]
    fn flat_order_to_item_idx_single() {
        let mut p = Project::new();
        p.patterns[0].repeat = 3;
        p.patterns.push(Pattern::new("Pattern 02".into(), 1, 16));
        p.arranger.push(ArrangerItem::Single { pattern_idx: 1 });
        assert_eq!(p.flat_order_to_item_idx(0), (0, 0));
        assert_eq!(p.flat_order_to_item_idx(2), (0, 0));
        assert_eq!(p.flat_order_to_item_idx(3), (1, 0));
    }

    #[test]
    fn flat_order_to_item_idx_group() {
        let mut p = Project::new();
        p.patterns[0].repeat = 2;
        p.patterns.push(Pattern::new("Pattern 02".into(), 1, 16));
        p.patterns[1].repeat = 3;
        p.arranger = vec![ArrangerItem::Group {
            name: "G1".into(),
            color: None,
            repeat: 2,
            pattern_indices: vec![0, 1],
            clone_id: None,
            collapsed: false,
        }];
        assert_eq!(p.flat_order_to_item_idx(0), (0, 0));
        assert_eq!(p.flat_order_to_item_idx(1), (0, 0));
        assert_eq!(p.flat_order_to_item_idx(2), (0, 1));
        assert_eq!(p.flat_order_to_item_idx(4), (0, 1));
        assert_eq!(p.flat_order_to_item_idx(5), (0, 0));
    }

    #[test]
    fn item_idx_to_flat_start_basic() {
        let mut p = Project::new();
        p.patterns[0].repeat = 3;
        p.patterns.push(Pattern::new("Pattern 02".into(), 1, 16));
        p.arranger.push(ArrangerItem::Single { pattern_idx: 1 });
        assert_eq!(p.item_idx_to_flat_start(0), 0);
        assert_eq!(p.item_idx_to_flat_start(1), 3);
    }
}
