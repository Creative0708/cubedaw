# Stuff to do for MVP

- [ ] Ui for track add/remove
- [ ] Ui for sections in track tab
- [ ] Saving/loading (just dump State & UiState via serde-json, probably)

- [ ] More nodes

  - [ ] how LFO?
    - [ ] Either merge LFO with the oscillator (<-- probably best solution) or make a new node
      - Most user-ergonomic: make the frequency slider go below 0 into the lfo range

- [ ] convert `log` to `tracing`

# Stuff to do after MVP

- [ ] Implement stereo sound
  - Not everything is stereo, so this would be locked behind implementing different types of sockets
- [ ] Implement different types of sockets
- uuughhhghhghhghgghhghghg
- Optimize everything
