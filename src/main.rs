#![warn(clippy::all)]

use std::collections::VecDeque;
use lab4::{
    checksum::Checksum, idea::IdeaGenerator, package::PackageDownloader, student::Student, Event,
};
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread::spawn;

struct Args {
    pub num_ideas: usize,
    pub num_idea_gen: usize,
    pub num_pkgs: usize,
    pub num_pkg_gen: usize,
    pub num_students: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let num_ideas = args.get(1).map_or(Ok(80), |a| a.parse())?;
    let num_idea_gen = args.get(2).map_or(Ok(2), |a| a.parse())?;
    let num_pkgs = args.get(3).map_or(Ok(4000), |a| a.parse())?;
    let num_pkg_gen = args.get(4).map_or(Ok(6), |a| a.parse())?;
    let num_students = args.get(5).map_or(Ok(6), |a| a.parse())?;
    let args = Args {
        num_ideas,
        num_idea_gen,
        num_pkgs,
        num_pkg_gen,
        num_students,
    };

    hackathon(&args);
    Ok(())
}

fn per_thread_amount(thread_idx: usize, total: usize, threads: usize) -> usize {
    let per_thread = total / threads;
    let extras = total % threads;
    per_thread + (thread_idx < extras) as usize
}

fn txt_to_lines(path: &str) -> VecDeque<String> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let mut vecdeque = VecDeque::new();

    for line in reader.lines() {
        let line = line.unwrap();
        vecdeque.push_back(line);
    }
    vecdeque.clone()
}

fn hackathon(args: &Args) {
    // Use message-passing channel as event queue
    let (send, recv) = unbounded::<Event>();
    let mut threads = Vec::new();
    let mut pkg_downloader_threads = VecDeque::new();
    // Checksums of all the generated ideas and packages
    let mut idea_checksum = Arc::new(Mutex::new(Checksum::default()));
    let mut pkg_checksum = Checksum::default();
    // Checksums of the ideas and packages used by students to build ideas. Should match the
    // previous checksums.
    let mut student_idea_checksum = Arc::new(Mutex::new(Checksum::default()));
    let mut student_pkg_checksum = Arc::new(Mutex::new(Checksum::default()));

    // Spawn student threads
    for i in 0..args.num_students {
        let mut student = Student::new(i, Sender::clone(&send), Receiver::clone(&recv));
        let student_idea_checksum = Arc::clone(&student_idea_checksum);
        let student_pkg_checksum = Arc::clone(&student_pkg_checksum);
        let thread = spawn(move || student.run(student_idea_checksum, student_pkg_checksum));
        threads.push(thread);
    }

    // Spawn package downloader threads. Packages are distributed evenly across threads.
    let pkgs = Arc::new(txt_to_lines("data/packages.txt"));
    let mut start_idx = 0;
    for i in 0..args.num_pkg_gen {
        let num_pkgs = per_thread_amount(i, args.num_pkgs, args.num_pkg_gen);
        let mut downloader = PackageDownloader::new(
            pkgs.clone(),
            start_idx,
            num_pkgs,
            Sender::clone(&send),
        );
        start_idx += num_pkgs;

        let thread = spawn(move || downloader.run());
        pkg_downloader_threads.push_back(thread);
    }
    assert_eq!(start_idx, args.num_pkgs);

    // Spawn idea generator threads. Ideas and packages are distributed evenly across threads. In
    // each thread, packages are distributed evenly across ideas.
    let mut start_idx = 0;
    for i in 0..args.num_idea_gen {
        let num_ideas = per_thread_amount(i, args.num_ideas, args.num_idea_gen);
        let num_pkgs = per_thread_amount(i, args.num_pkgs, args.num_idea_gen);
        let num_students = per_thread_amount(i, args.num_students, args.num_idea_gen);
        let generator = IdeaGenerator::new(
            start_idx,
            num_ideas,
            num_students,
            num_pkgs,
            Sender::clone(&send),
        );
        let idea_checksum = Arc::clone(&idea_checksum);
        start_idx += num_ideas;

        let thread = spawn(move || generator.run(idea_checksum));
        threads.push(thread);
    }
    assert_eq!(start_idx, args.num_ideas);

    // Join all threads
    threads.into_iter().for_each(|t| t.join().unwrap());
    pkg_downloader_threads.into_iter().for_each(|t| {
        let checksum = t.join().unwrap();
        pkg_checksum.update(checksum);
    });

    let idea = Arc::get_mut(&mut idea_checksum).unwrap().get_mut().unwrap();
    let student_idea = Arc::get_mut(&mut student_idea_checksum)
        .unwrap()
        .get_mut()
        .unwrap();
    // let pkg = Arc::get_mut(&mut pkg_checksum).unwrap().get_mut().unwrap();
    let student_pkg = Arc::get_mut(&mut student_pkg_checksum)
        .unwrap()
        .get_mut()
        .unwrap();

    println!("Global checksums:\nIdea Generator: {}\nStudent Idea: {}\nPackage Downloader: {}\nStudent Package: {}", 
        idea, student_idea, pkg_checksum, student_pkg);
}
