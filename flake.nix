{
  description = "Runtime-configurable password picker with multiple compiled backends";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forEachSystem = f:
        nixpkgs.lib.genAttrs systems (system:
          f {
            inherit system;
            pkgs = import nixpkgs { inherit system; };
          });
    in
    {
      packages = forEachSystem ({ pkgs, ... }:
        let
          runtimeTools = with pkgs; [
            bemenu
            fuzzel
            keepassxc
            libnotify
            pass
            passage
            wofi
            wtype
            wl-clipboard
            xclip
          ];
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "pmenu";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [
              makeWrapper
              pkg-config
            ];

            postFixup = ''
              wrapProgram $out/bin/pmenu \
                --prefix PATH : ${pkgs.lib.makeBinPath runtimeTools}
            '';
          };
        });

      devShells = forEachSystem ({ pkgs, ... }:
        let
          runtimeTools = with pkgs; [
            bemenu
            fuzzel
            keepassxc
            libnotify
            pass
            passage
            wofi
            wtype
            wl-clipboard
            xclip
          ];
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              pkg-config
              rust-analyzer
              rustc
              rustfmt
            ] ++ runtimeTools;
          };
        });
    };
}
