use std::collections::VecDeque;
use super::checksum::Checksum;
use super::Event;
use crossbeam::channel::Sender;
use std::sync::{Arc};

pub struct Package {
    pub name: String,
}

pub struct PackageDownloader {
    pkgs: Arc<VecDeque<String>>,
    pkg_start_idx: usize,
    num_pkgs: usize,
    event_sender: Sender<Event>,
    pkg_checksum: Checksum
}

impl PackageDownloader {
    pub fn new(pkgs: Arc<VecDeque<String>>, pkg_start_idx: usize, num_pkgs: usize, event_sender: Sender<Event>) -> Self {
        Self {
            pkgs,
            pkg_start_idx,
            num_pkgs,
            event_sender,
            pkg_checksum: Checksum::default()
        }
    }

    pub fn run(&mut self) -> Checksum {
        // Generate a set of packages and place them into the event queue
        // Update the package checksum with each package name
        for i in 0..self.num_pkgs {
            let index = (self.pkg_start_idx + i) % self.pkgs.len();
            let name = self.pkgs[index].clone();

            self.pkg_checksum
                .update(Checksum::with_sha256(&name));

            self.event_sender
                .send(Event::DownloadComplete(Package { name }))
                .unwrap();
        }
        self.pkg_checksum.clone()
    }
}
