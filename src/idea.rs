use std::collections::VecDeque;
use super::checksum::Checksum;
use super::Event;
use crossbeam::channel::Sender;
use std::sync::Arc;

pub struct Idea {
    pub name: String,
    pub num_pkg_required: usize,
}

pub struct IdeaGenerator {
    ideas: Arc<VecDeque<(String, String)>>,
    idea_start_idx: usize,
    num_ideas: usize,
    // num_students: usize,
    num_pkgs: usize,
    event_sender: Sender<Option<Idea>>,
    pkg_per_idea: usize,
    extra_pkgs: usize,
    idea_checksum: Checksum,
}

impl IdeaGenerator {
    pub fn new(
        ideas: Arc<VecDeque<(String, String)>>,
        idea_start_idx: usize,
        num_ideas: usize,
        // num_students: usize,
        num_pkgs: usize,
        event_sender: Sender<Option<Idea>>,
    ) -> Self {
        Self {
            ideas,
            idea_start_idx,
            num_ideas,
            // num_students,
            num_pkgs,
            event_sender,
            pkg_per_idea: num_pkgs / num_ideas,
            extra_pkgs: num_pkgs % num_ideas,
            idea_checksum: Checksum::default(),
        }
    }

    // Idea names are generated from cross products between product names and customer names
    fn get_next_idea_name(&self, idx: usize) -> String {
        let pair = &self.ideas[idx % self.ideas.len()];
        format!("{} for {}", pair.0, pair.1)
    }

    pub fn run(&mut self) -> Checksum {
        // Generate a set of new ideas and place them into the event-queue
        // Update the idea checksum with all generated idea names
        for i in 0..self.num_ideas {
            let name = self.get_next_idea_name(self.idea_start_idx + i);
            let extra = (i < self.extra_pkgs) as usize;
            let num_pkg_required = self.pkg_per_idea + extra;
            let idea = Idea {
                name,
                num_pkg_required,
            };

            // idea_checksum
            //     .lock()
            //     .unwrap()
            //     .update(Checksum::with_sha256(&idea.name));
            self.idea_checksum.update(Checksum::with_sha256(&idea.name));

            self.event_sender.send(Some(idea)).unwrap();
        }
        self.idea_checksum.clone()
    }
}
