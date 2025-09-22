{ ... }:
{
  perSystem =
    { config, pkgs, ... }:
    let
      cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
    in
    {
      packages.wristvault = pkgs.rustPlatform.buildRustPackage {
        pname = "wristvault";
        version = cargoToml.package.version;

        src = ../.;

        cargoLock = {
          lockFile = ../Cargo.lock;
        };

        meta = with pkgs.lib; {
          description = cargoToml.package.description;
          homepage = "https://github.com/nilp0inter/wristvault";
          license = licenses.gpl3Plus;
          maintainers = with maintainers; [ nilp0inter ];
          mainProgram = "wristvault";
          platforms = platforms.linux ++ platforms.darwin;
        };
      };

      # Make wristvault the default package
      packages.default = config.packages.wristvault;
    };
}
