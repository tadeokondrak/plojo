{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [ pkg-config ];
  buildInputs = with pkgs; [ libudev ];
  inputsFrom = with pkgs; [ ];
  hardeningDisable = [ "all" ];
}
