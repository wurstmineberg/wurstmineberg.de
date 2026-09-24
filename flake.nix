{
    inputs.flake.url = "github:fenhl/flake";
    outputs = attrs: attrs.flake.lib {
        devShells.default = { pkgs, ... }: {
            packages = with pkgs; [
                cargo
                clang
            ];
        };
    };
}
