# Stuff to do for MVP

- [ ] Saving/loading (just dump State & UiState via serde-json, probably)

- [ ] More nodes

  - [ ] how LFO?
    - [ ] Either merge LFO with the oscillator (<-- probably best solution) or make a new node
      - Most user-ergonomic: make the frequency slider go below 0 into the lfo range
  - [ ] Distortion (hard & soft)
    - [ ] Optionally, a waveshaper (take blender's node curve editor)
  - [ ] Chorus (!!!)
  - [ ] Delay (Echo)
  - [ ] Filters (If nothing else, the ones at https://webaudio.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html)
  - [ ] Simple compressor
    - Attack, Release, Ratio, Threshold, Gain
  - [ ] Envelope
  - [ ] EQ
  - [ ] Bitcrusher

- [x] convert `log` to `tracing`

- [ ] flesh out track tab

  - [ ] Ui for track add/remove
  - [ ] Ui for clips in track tab
    - [ ] Ui for notes in secions in track tab

- [ ] add a `kick()` function to the plugin api to give plugins more control over the note finish detection

- [ ] DOCUMENTATION DOCUMENTATION DOCUMENTATION DOCUMENTATION

- [ ] remove the entirety of `cubedaw-command` (move `StateCommand` to `cubedaw-worker` and use [dyn upcasting](https://github.com/rust-lang/rust/issues/65991) to coerce `UiStateCommand` (or just have the `StateCommandWrapper` do that))

# Stuff to do after MVP

- [ ] Implement stereo sound (yes, MVP is gonna be mono :/)
  - Not everything is stereo, so this would be locked behind implementing different types of sockets
- [ ] Implement different types of sockets
- uuughhhghhghhghgghhghghg
- Optimize everything
  - [ ] change hashmaps to more efficient data structures
  - [ ] add a hashset for selected notes/clips/whatever that's kept in sync with everything
- [ ] Add tempo automation
  - Have to use a curve that's easily integratable so we don't run into timing performance issues (see [https://ardour.org/timing.html])
- [ ] Add node graph paralellization (compile node )
