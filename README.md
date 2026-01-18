# nd

POC for an easy and fast `nix develop` workflow that works well with
tmux.

## usage

- `nd build` or `nd b` will build a devShell.
- `nd shell` or `nd s` or `nd` will start a devShell.
- `nd run something` or `nd r something` will run `something` inside a
  devShell.

By default it looks for any flake in the current folder or a parent
folder, and it uses the default shell (`#default`).

All of the above commands can also be started with `nd at flakeref ...`
to use a different flake. For example
`nd at ~/fun/project run cargo --version` will use that devShell even
when you are in a different folder. Only local flake references are
allowed:

- `../somewhere/here`
- `../somewhere/here#name`
- `path:../somewhere/here`

Additionally, the flake reference `-` is allowed. It means no flake, and
`nd run something` runs `something` normally, without a devShell.

Alternatively, if there is an env var `nd_env`, then you can also use
`nd env ...` or `nd e ...` to use the flake reference pointed at by
`nd_env`. This is mostly useful for `tmux`, see below.
`nd env run something` just runs `something` normally, if there is no
`nd_env`. If `nd_env` is not set, `nd env ...` will behave like
`nv at - ...` and just work.

The cached devShell is in `.nd`. You have to explicitely rebuild. It
will only build automatically if there is no cached devShell yet. The
cached devShell's `.nd` will always be at the referenced flake's
location, which is not necessarily your project's root.

## tmux

Set the `default-shell` of tmux to `tmux-nd-shell`. In tmux.conf:

``` tmux
run-shell -b 'tmux set-option -g default-shell `which tmux-nd-shell`'
```

Then run a tmux session with the env var `nd_env` set to the project's
flake reference. Every shell inside that tmux session will now be a
devShell.

``` bash
tmux new-session -e nd_env=$HOME/fun/project
```

Note that you cannot use `~` here, it won't be expanded. That is `zsh`
behaviour, not `nd` behaviour.

Or use `nd tmux` to start an `nd`-enabled session. Note that the
session's folder will be the current folder, but the session's nd flake
can be from a different place when using `nd at ~/other/place tmux`.

If you have things you run on shortcuts within tmux, and if they should
(sometimes) use `nd`, a convenient way is to map it to
`nd env run something`. It will work no matter if `nd` is enabled or
not.

``` bash
tmux bind-key g run-shell -b 'nd env run something --arg value'
```
