class Buzzer < Formula
  desc "Wait your turn for a shared thing (a test device, an editor), then get buzzed"
  homepage "https://github.com/alon-talmi/buzzer"
  url "https://github.com/alon-talmi/buzzer/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "6c00c831a6a8c3795c92b2ad5861b5c68cfa5b852e34da4cc61ec20cf96c5368"
  license "MIT"
  head "https://github.com/alon-talmi/buzzer.git", branch: "main"

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
