# nd - A Fast Nix Develop Wrapper

Are you using nixos and flakes? In that case: Good news, `nd` might be
for you!

This `nd` is a wrapper around `nix develop` and makes it lightning fast,
so fast in fact that you won't notice the difference between starting a
new shell and starting a new devShell in, eg, your `tmux`.

The workflow is very similar to how you would use a python virtualenv's
`activate` or `uv run`.

A typical `tmux` usage goes like:

- `nd tmux` - start a `tmux` session in project with a `flake.nix`.
- Now every new pane you open in `tmux` is automatically a devShell, no
  delay.
- `nd build` will rebuild if the devShell has changed.

If you don't work in `tmux`, you might just `nd shell` to spin up a
devShell, or maybe `nd run -- nvim` to run some other binary inside a
devShell.

I won't lie to you, it is a pretty thin wrapper around `nix develop`,
and depending on how much you care about the responsiveness of your
terminal, you could be totally fine with plain `nix develop`.

## The Approach

It uses nix's own profile-functionality to build and cache devShell.
They will be saved in `./.nd/dev-?-link` symlinks. This also prevents it
from being garbage collected out of the nix store. See [garbage
collector
roots](https://nix.dev/manual/nix/2.34/package-management/garbage-collector-roots.html).

So, `nd build` might be slow, depending on your flake, but `nd run` will
be fast, as there is no `nix` involved anywhere, the profile is loaded
in an instant. You could even manually use `./.nd/run` instead of the
`nd *` commands if you wanted. That `./.nd/run` is akin to a python
virtualenv's `activate`.

## So Fast - What's The Catch?

Unlike some other `nix develop` wrappers, `nd` doesnt make any attempt
to automatically `nd build` when the flake has changed. Such an
automatic build has two problems that I like to avoid:

- Unexpected rebuilds can introduce unwanted delays in your workflow.
- Detecting when a rebuild is needed is notoriously difficult with
  flakes. Ultimately, you have to build the context and evaluate. At
  that point you already have a noticeable delay. Other solutions that
  don't build the context first are heuristics that don't always work.

Therefore, the catch is: You yourself have to `nd build` whenever
needed. `nd` will warn you when your last build is very old, or when the
`flake.lock` has changed, but it will never rebuild for you unasked.

Nothing of course stops you from wrapping `nd` again with some smart
rebuild logic for your current project. A generic automatic rebuild is
not feasible with speed in mind, but the game changes when it is only
for a very specific project layout.

## Install

This is a flake. If you use nixos, you should know how to use it.

Add it to the flake inputs:

``` nix
nd.url = "github:dkuettel/nd/main";
```

And then add it to the installed packages:

``` nix
environment.systemPackages = [nd.packages.${pkgs.stdenv.hostPlatform.system}.default];
```

Or with `home-manager`:

``` nix
home-manager.users.USER.home.packages = [nd.packages.${pkgs.stdenv.hostPlatform.system}.default];
```

Optionally, you can use the flake's output `shell` to add functionality
to `zsh`. In your `.zshrc` or similar, first source
`${nd.packages.${pkgs.stdenv.hostPlatform.system}.shell}/share/nd/activate.zsh`
and then you can use the `zsh` function `__nd_status` in the prompt, for
example:

``` zsh
export PS1='... $(__nd_status) ...'
```

This will show you a small warning when the current shell is using an
old devShell profile due to a recent rebuild.

## Tmux

In order to use it with `tmux`, configure it to use `nd` to start new
panes:

``` tmux
set-option -g default-command nd-tmux-default-command
```

This will make it respect the env var `nd_env` when in tmux. If `nd_env`
points to the folder of a `flake.nix`, then that devShell will be used
for tmux. If `nd_env` is set to `-` then no devShell will be used. It is
an error if you forget to set `nd_env`.

To run tmux bindings that should execute something in a devShell, use,
eg:

``` tmux
bind-key g run-shell -b 'nd-tmux-run something --opt arg1 arg2'
```

## If You Are Still Here

Only the following flake references are supported:

- Plain local paths to a folder with a `flake.nix` in it.
- `path:some/folder` to use a path (ie, not git) flake, see [flake
  references](https://nix.dev/manual/nix/2.18/command-ref/new-cli/nix3-flake.html#path-like-syntax).
- `some/folder#name` to use a non-default devShell from a flake.
- `-` to indicate no devShell, ie, `nd run --at=- nvim` will just be
  `nvim`.

Note in particular that currenly remote flakes (like
`github:someone/something/branch`) are not supported.

The main binary is `nd`, but there is also:

- `nd-run` short for `nd --silent run --build-if-missing -- ...`.
  Aliased to `nr` makes it easy to run things with different devShells
  on the spot.
- `nd-tmux-default-command` and `nd-tmux-run` as seen above for the
  `tmux` configuration.

As always,\
`nd --help` is your friend.\
Sincerely,\
end of data.
