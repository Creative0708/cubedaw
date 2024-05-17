
- don't share data across threads, anything that blocks for a long time is usually a bad idea
  - ui thread blocking can be (is) very bad
  - instead, use channels to communicate and store separate instances of a state that is synchronized whenever a thread finishes computation
