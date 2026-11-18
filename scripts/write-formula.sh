#!/bin/sh
# Usage: write-formula.sh <tag> <folder holding the four buzzer-<tag>-<target>.tar.gz files>
set -eu
tag="$1"; dir="$2"; version="${tag#v}"
base="https://github.com/alon-talmi/buzzer/releases/download/$tag"
sha() { shasum -a 256 "$dir/buzzer-$tag-$1.tar.gz" | cut -d' ' -f1; }
cat > Formula/buzzer.rb <<RUBY
class Buzzer < Formula
  desc "Wait your turn for a shared thing (a test device, an editor), then get buzzed"
  homepage "https://github.com/alon-talmi/buzzer"
  version "$version"
  license "MIT"

  on_macos do
    on_arm do
      url "$base/buzzer-$tag-aarch64-apple-darwin.tar.gz"
      sha256 "$(sha aarch64-apple-darwin)"
    end
    on_intel do
      url "$base/buzzer-$tag-x86_64-apple-darwin.tar.gz"
      sha256 "$(sha x86_64-apple-darwin)"
    end
  end

  on_linux do
    on_arm do
      url "$base/buzzer-$tag-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "$(sha aarch64-unknown-linux-gnu)"
    end
    on_intel do
      url "$base/buzzer-$tag-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "$(sha x86_64-unknown-linux-gnu)"
    end
  end

  def install
    bin.install "buzzer"
  end

  test do
    ENV["BUZZER_HOME"] = testpath
    assert_match "YOUR TURN", shell_output("#{bin}/buzzer wait smoke --label t")
  end
end
RUBY
