{ pkgs }:
let
  noto-fonts-cjk = pkgs.fetchFromGitHub {
    owner = "googlefonts";
    repo = "noto-cjk";
    rev = "1c7ca85cb5195a3332e18c2b5cfe196ffb084e72";
    sha256 = "sha256-541hsYHqjBYTBEg7ooGfX1+hJLo4QouQnVOIq8UzN7Y=";
    sparseCheckout = [ "Sans/OTC" ];
  };
in {
  bold = "${noto-fonts-cjk}/Sans/OTC/NotoSansCJK-Black.ttc";
  light = "${noto-fonts-cjk}/Sans/OTC/NotoSansCJK-Light.ttc";
}
