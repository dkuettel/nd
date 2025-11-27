# nd

POC for an easy and fast `nix develop` workflow.

## usage

-   `nd build` or `nd b` will build a devShell.
-   `nd shell` or `nd s` or `nd` will start a devShell.
-   `nd run ...` or `nd r ...` will run someting inside devShell.

By default it looks for any flake in the current folder or a parent
folder. All of the above commands can also be startsd with
`nd at flakeref ...` to use a different flake. For example
`nd at ~/fun/project run cargo --version` will use that devShell even
when you are in a different folder. It takes normal flake references,
just like `nix develop` does.

The cache devShell is in `.nd`. You have to explicitely rebuild. It will
only build automatically if there is no cached devShell yet.

## tmux

Set the `default-shell` of tmux to `tmux-nd-shell`. In tmux.conf:

``` tmux
run-shell -b 'tmux set-option -g default-shell `which tmux-nd-shell`'
```

Then run a tmux session with the env var `nd` set to the project's flake
folder. Every shell inside that tmux session will now be a devShell.

``` bash
tmux new-session -e nd=~/fun/project
```
