class Buzzer < Formula
  desc "Wait your turn for a shared thing (a test device, an editor), then get buzzed"
  homepage "https://github.com/OWNER/buzzer"
  url "https://github.com/OWNER/buzzer/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "REPLACE_WITH_TARBALL_SHA256"
  license "MIT"
  head "https://github.com/OWNER/buzzer.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    ENV["BUZZER_HOME"] = testpath
    out = shell_output("#{bin}/buzzer wait smoke --label t")
    assert_match "YOUR TURN", out
  end
end
