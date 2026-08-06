
function __nd_status {
    if [[ ! -v nd ]]; then
        echo good
        return
    fi

    # first entry (could be multiple)
    local nds=(${(s.:.)nd})
    local at=$nds[1]

    # NOTE NIX_GCROOT is not set when using print-dev-env, so we use our own
    if [[ -e $nd/.nd/dev && $(realpath $nd/.nd/dev) == ${nd_nix:-} ]]; then
        echo good
        return
    fi

    echo old
}
