# [PR] Removed redundant file reads, locks, and checksum hex encodings, and refactored message-passing channels

# Summary
To improve the performance of the original hackathon, we removed redundant file reads such 
that the txt files only have to read once in `main.rs`. To avoid using locks to prevent 
contention, we used separate message-passing channels for packages and ideas. Additionally,
we make threads compute their checksums locally and combine them after all are processed,
to reduce idle waiting time from locking a global checksum for every update. Lastly, we 
reduced the hex encoding / decoding in `checksum.rs` when updating a checksum, as they are
only essential for printouts.

# Technical details

### Removed redundant file reads and improving package name retrieval
To eliminate the need for re-reading entire data files on every generation of a package or idea,
we moved the file reads to the main thread when the program starts. This data is loaded into 
`VecDeque`s and shared using `Arc` with threads which require the data. This allows for indexed
access for packages and ideas.
For obtaining package names (in the `run()` function in `PackageDownloader`) in particular, 
using the `cycle()` method to obtain the index for the package name is expensive. To reduce 
the time consumed by `Iterator::nth`, we use modular arithmetic to obtain the required index instead.

### Removed redundant locks to reduce idle waiting time (`crossbeam_utils::backoff::Backoff::spin`)
We perform XORs for idea and package hashes locally for each thread. These values are accumulated
for each thread. All the local checksums will be joined in the main thread after each thread 
terminates, and the final global checksums (`idea_checksum`, `pkg_checksum`, `student_idea`, 
`student_pkg`) are obtained. 

### Removed redundant checksum hex encodings
Hex encoding / decoding is unnecessary when updating a checksum, this is only required in printouts. 
Hence, we use a `Vec<u8>` to store raw bytes instead of the hex-encoded bytes in a `String`.

### Refactored message-passing channels to reduce contention
Significant contention arises from the single message-passing channel which the producers 
(`PackageDownloader` and `IdeaGenerator`) and consumer (`Student`) share. Students may have to 
push ideas back into the channel, which increases contention due to more push and pop operations.
We create separate Package `(pkg_send, pkg_recv)` and Idea `(idea_send, idea_recv)` channels
to reduce this contention. To terminate student threads, poison pills (`Event::OutOfIdeas`) 
are pushed into the Idea channel.

# Testing for correctness
We compare our outputs from this optimization and the original code by performing 
`cargo run --release`. The four global checksums (`idea_checksum`, `student_idea`, `pkg_checksum`, 
and `student_pkg`) were identical for all runs. This indicates that the optimized code is
functionally correct.

# Testing for performance

We report consistently faster performance for our optimized code, especially as the `num_pkgs`
increases. We find that running `hyperfine -i "target/release/lab4 80 2 100000 6 6" --warmup=3`
(num_ideas = 80, num_idea_gen = 2, num_pkgs = 100000, num_pkg_gen = 6, num_students = 6), 
required 31.0 ms ± 2.9 ms for the optimized code, and 28.090 s ± 0.423 s for the original code.
This amounts to a ~906x speed up, benchmarked locally (Apple MacBook Pro 2021, 
with Apple M1 Pro).

From the flamegraph of the original code, we observed that significant time was spent sending
and receiving over the Crossbeam channel. After adding a new channel to separate Package and
Idea events, we observe a significant decrease in the proportion of time spent communicating 
over the channels. We further decrease the time spent by `crossbeam_utils::backoff::Backoff::spin`
by performing the XORs locally for each thread, then joining them at the end. 

Next, to reduce the considerable time spent by the Iterator, reflected on the original flamegraph, we removed the
need to call the `cycle()` method as the `packages.txt` is re-read on every generation of a 
package. We did this by reading all the data files once in the main thread when the program starts, 
then using modular arithmetic to obtain the required indices for the package name.

Moreover, the flamegraph indicated that hex encoding / decoding took up a considerable duration.
Removing the need for these conversions during update reduced the proportion of time taken used
by the Checksum methods.

Lastly, since intermediate printouts are not required, we eliminated them to save the time
spent locking `stdout` when an idea has been built and the student thread wants to print it out.

(Note: We did not quote exact percentages from the flamegraph because there were quite a few
`[unknown]`s, which negates accuracy)