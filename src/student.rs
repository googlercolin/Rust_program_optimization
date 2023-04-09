use super::{checksum::Checksum, idea::Idea, package::Package, Event};
use crossbeam::channel::Receiver;
use std::io::Write;
use std::collections::VecDeque;
use std::io;

// pub struct Student {
//     id: usize,
//     idea: Option<Idea>,
//     pkgs: VecDeque<Package>,
//     skipped_idea: bool,
//     event_sender: Sender<Event>,
//     event_recv: Receiver<Event>,
// }

pub struct Student {
    id: usize,
    pkg_recv: Receiver<Package>,
    pkg_checksum: Checksum,
    idea_recv: Receiver<Option<Idea>>,
    idea_checksum: Checksum,
    build_msg: String,
}

impl Student {
    pub fn new(id: usize, pkg_recv: Receiver<Package>, idea_recv: Receiver<Option<Idea>>) -> Self {
        Self {
            id,
            pkg_recv,
            pkg_checksum: Checksum::default(),
            idea_recv,
            idea_checksum: Checksum::default(),
            build_msg: String::new(),
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

        // if let Some(ref idea) = self.idea {
        //     // Can only build ideas if we have acquired sufficient packages
        //     let pkgs_required = idea.num_pkg_required;
        //     if pkgs_required <= self.pkgs.len() {
        //         let (mut idea_checksum, mut pkg_checksum) =
        //             (idea_checksum.lock().unwrap(), pkg_checksum.lock().unwrap());
        //
        //         // Update idea and package checksums
        //         // All of the packages used in the update are deleted, along with the idea
        //         idea_checksum.update(Checksum::with_sha256(&idea.name));
        //         let pkgs_used = self.pkgs.drain(0..pkgs_required).collect::<VecDeque<_>>();
        //         for pkg in pkgs_used.iter() {
        //             pkg_checksum.update(Checksum::with_sha256(&pkg.name));
        //         }
        //
        //         // We want the subsequent prints to be together, so we lock stdout
        //         let stdout = stdout();
        //         let mut handle = stdout.lock();
        //         writeln!(handle, "\nStudent {} built {} using {} packages\nIdea checksum: {}\nPackage checksum: {}",
        //             self.id, idea.name, pkgs_required, idea_checksum, pkg_checksum).unwrap();
        //         for pkg in pkgs_used.iter() {
        //             writeln!(handle, "> {}", pkg.name).unwrap();
        //         }
        //
        //         self.idea = None;
        //     }
        // }
    }

    pub fn run(&mut self) -> (Checksum, Checksum) {
        let mut pkgs:VecDeque<Package> = VecDeque::new();
        loop {
            // let event = self.event_recv.recv().unwrap();
            let received_idea = self.idea_recv.recv().unwrap();
            match received_idea {
                Some(idea) => {
                    for _ in 0..idea.num_pkg_required {
                        pkgs.push_back(self.pkg_recv.recv().unwrap());
                    }
                    self.build_idea(&pkgs, idea);
                    pkgs.clear();
                }

                None => {
                    write!(io::stdout(), "{}", self.build_msg).unwrap();
                    return (self.idea_checksum.clone(), self.pkg_checksum.clone())
                }

                // Event::NewIdea(idea) => {
                //     // If the student is not working on an idea, then they will take the new idea
                //     // and attempt to build it. Otherwise, the idea is skipped.
                //     if self.idea.is_none() {
                //         self.idea = Some(idea);
                //         self.build_idea(&idea_checksum, &pkg_checksum);
                //     } else {
                //         self.event_sender.send(Event::NewIdea(idea)).unwrap();
                //         self.skipped_idea = true;
                //     }
                // }
                //
                // Event::DownloadComplete(pkg) => {
                //     // Getting a new package means the current idea may now be buildable, so the
                //     // student attempts to build it
                //     self.pkgs.push_back(pkg);
                //     self.build_idea(&idea_checksum, &pkg_checksum);
                // }
                //
                // Event::OutOfIdeas => {
                //     // If an idea was skipped, it may still be in the event queue.
                //     // If the student has an unfinished idea, they have to finish it, since they
                //     // might be the last student remaining.
                //     // In both these cases, we can't terminate, so the termination event is
                //     // deferred ti the back of the queue.
                //     if self.skipped_idea || self.idea.is_some() {
                //         self.event_sender.send(Event::OutOfIdeas).unwrap();
                //         self.skipped_idea = false;
                //     } else {
                //         // Any unused packages are returned to the queue upon termination
                //         for pkg in self.pkgs.drain(..) {
                //             self.event_sender
                //                 .send(Event::DownloadComplete(pkg))
                //                 .unwrap();
                //         }
                //         return;
                //     }
                // }
            }
        }
    }
}
