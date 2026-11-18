class Buzzer < Formula
  desc "Wait your turn for a shared thing (a test device, an editor), then get buzzed"
  homepage "https://github.com/alon-talmi/buzzer"
  version "0.1.6"
  license "MIT"

  depends_on arch: :arm64
  depends_on :macos

  url "https://github.com/alon-talmi/buzzer/releases/download/v0.1.6/buzzer-v0.1.6-aarch64-apple-darwin.tar.gz"
  sha256 "b0dfff0e521d4484d2ea332c3705effa6cb8c956e995d8aaf7b035c4d40af471"

  def install
    bin.install "buzzer"
  end

  test do
    ENV["BUZZER_HOME"] = testpath
    assert_match "YOUR TURN", shell_output("#{bin}/buzzer wait smoke --label t")
  end
end
