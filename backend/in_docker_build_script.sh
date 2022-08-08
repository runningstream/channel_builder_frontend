export PATH="$HOME/.cargo/bin:$PATH"

if [ "$1" = "debug" ]; then
    make
    exit
fi

make release
