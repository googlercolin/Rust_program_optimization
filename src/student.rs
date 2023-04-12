use super::{checksum::Checksum, idea::Idea, package::Package, Event};
use crossbeam::channel::Receiver;
use std::io::Write;
use std::collections::VecDeque;
use std::io;

pub struct Student {
    id: usize,
    pkg_recv: Receiver<Event>,
    pkg_checksum: Checksum,
    idea_recv: Receiver<Event>,
    idea_checksum: Checksum,
    build_msg: String,
    idea: Option<Idea>,
    pkgs: VecDeque<Package>
}

impl Student {
    pub fn new(id: usize, pkg_recv: Receiver<Event>, idea_recv: Receiver<Event>) -> Self {
        Self {
            id,
            pkg_recv,
            pkg_checksum: Checksum::default(),
            idea_recv,
            idea_checksum: Checksum::default(),
            build_msg: String::new(),
            idea: None,
            pkgs: VecDeque::new()
        }
    }

    fn build_idea(
        &mut self,
        pkgs: &VecDeque<Package>,
        idea: Idea,
    ) {
        // Update idea and package checksums
        self.idea_checksum.update(Checksum::with_sha256(&idea.name));
        for pkg in pkgs {
            self.pkg_checksum.update(Checksum::with_sha256(&pkg.name));
        }
        let pkgs_required = pkgs.len();
        let mut build_str = format!("\nStudent {} built {} using {} packages\nIdea checksum: {}\nPackage checksum: {}\n",
                                    self.id, idea.name, pkgs_required, self.idea_checksum, self.pkg_checksum);
        for pkg in pkgs {
            build_str += &format!("> {}\n", pkg.name);
        }
        self.build_msg += &build_str;
        self.idea = None;
    }

    pub fn run(&mut self) -> (Checksum, Checksum) {
        let mut pkgs:VecDeque<Package> = VecDeque::new();
        loop {
            // let event = self.event_recv.recv().unwrap();
            let received_idea = self.idea_recv.recv().unwrap();
            match received_idea {

                Event::NewIdea(idea) => {
                    // If the student is not working on an idea, then they will take the new idea
                    // and attempt to build it. Otherwise, the idea is skipped.
                    for _ in 0..idea.num_pkg_required {
                        if let Event::DownloadComplete(pkg) = self.pkg_recv.recv().unwrap() {
                            pkgs.push_back(pkg);
                        }
                    }
                    self.idea = Some(idea.clone());
                    self.build_idea(&pkgs, idea);
                    pkgs.clear();
                }

                Event::DownloadComplete(pkg) => {
                    // Getting a new package means the current idea may now be buildable, so the
                    // student attempts to build it
                    self.pkgs.push_back(pkg);
                    self.build_idea(&pkgs, self.idea.clone().unwrap());
                }

                Event::OutOfIdeas => {
                    write!(io::stdout(), "{}", self.build_msg).unwrap();
                    return (self.idea_checksum.clone(), self.pkg_checksum.clone())
                }
            }
        }
    }
}
