
autoload -U add-zsh-hook

function __nd_prompt_precmd {
    if [[ ! -v nd ]]; then
        return
    fi

    # first entry (could be multiple)
    local nds=(${(s.:.)nd})
    local at=$nds[1]

    if [[ $(realpath $nd/.nd/dev) == ${NIX_GCROOT:-} ]]; then
        return
    fi

    echo
    print -P -- '%F{1}%S'"nd: There is a newer profile at $at"'.%s%f'
}

# runs just before the prompt after a command, but not on redraw
add-zsh-hook precmd __nd_prompt_precmd
