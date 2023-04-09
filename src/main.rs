#![warn(clippy::all)]

use std::collections::VecDeque;
use lab4::{checksum::Checksum, Event, idea::IdeaGenerator, package::PackageDownloader, student::Student};
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::thread::spawn;
use lab4::idea::Idea;
use lab4::package::Package;

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
    let amt= per_thread + (thread_idx < extras) as usize;
    amt
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

fn cross_product(products: VecDeque<String>, customers: VecDeque<String>) -> VecDeque<(String, String)> {
    let ideas = products.iter()
        .flat_map(|p| customers.iter().map(move |c| (p.clone(), c.clone())))
        .collect();
    ideas
}

fn get_ideas() -> VecDeque<(String, String)>{
    let products = txt_to_lines("data/ideas-products.txt");
    let customers = txt_to_lines("data/ideas-customers.txt");
    cross_product(products, customers)
}

fn hackathon(args: &Args) {
    // Use message-passing channel as pkg event queue
    let (pkg_send, pkg_recv) = unbounded::<Event>();
    // Use message-passing channel as idea event queue
    let (idea_send, idea_recv) = unbounded::<Event>();

    // Initialize threads
    let mut threads = VecDeque::new();
    let mut pkg_downloader_threads = VecDeque::new();
    let mut idea_gen_threads = VecDeque::new();

    // Checksums of all the generated ideas and packages
    let mut idea_checksum = Checksum::default();
    let mut pkg_checksum = Checksum::default();

    // Checksums of the ideas and packages used by students to build ideas. Should match the
    // previous checksums.
    let mut student_idea = Checksum::default();
    let mut student_pkg = Checksum::default();

    // Spawn student threads
    for i in 0..args.num_students {
        let mut student = Student::new(i, Receiver::clone(&pkg_recv), Receiver::clone(&idea_recv));
        let thread = spawn(move || student.run());
        threads.push_back(thread);
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
            Sender::clone(&pkg_send),
        );
        start_idx += num_pkgs;

        let thread = spawn(move || downloader.run());
        pkg_downloader_threads.push_back(thread);
    }
    assert_eq!(start_idx, args.num_pkgs);

    // Spawn idea generator threads. Ideas and packages are distributed evenly across threads. In
    // each thread, packages are distributed evenly across ideas.
    let ideas = Arc::new(get_ideas());
    let mut start_idx = 0;
    for i in 0..args.num_idea_gen {
        let num_ideas = per_thread_amount(i, args.num_ideas, args.num_idea_gen);
        let num_pkgs = per_thread_amount(i, args.num_pkgs, args.num_idea_gen);
        // let num_students = per_thread_amount(i, args.num_students, args.num_idea_gen);
        let mut generator = IdeaGenerator::new(
            ideas.clone(),
            start_idx,
            num_ideas,
            // num_students,
            num_pkgs,
            Sender::clone(&idea_send),
        );
        start_idx += num_ideas;

        let thread = spawn(move || generator.run());
        idea_gen_threads.push_back(thread);
    }
    assert_eq!(start_idx, args.num_ideas);

    // Join pkg downloader threads
    pkg_downloader_threads.into_iter().for_each(|t| {
        let checksum = t.join().unwrap();
        pkg_checksum.update(checksum);
    });

    // Join idea gen threads
    idea_gen_threads.into_iter().for_each(|t| {
        let checksum = t.join().unwrap();
        idea_checksum.update(checksum);
    });

    // Insert poison pills for students after ideas generated
    for _ in 0..args.num_students {
        idea_send.send(Event::OutOfIdeas);
    }

    // Join student threads
    threads.into_iter().for_each(|t| {
        let checksums = t.join().unwrap();
        student_idea.update(checksums.0);
        student_pkg.update(checksums.1);
    });

    println!("Global checksums:\nIdea Generator: {}\nStudent Idea: {}\nPackage Downloader: {}\nStudent Package: {}", 
        idea_checksum, student_idea, pkg_checksum, student_pkg);
}
