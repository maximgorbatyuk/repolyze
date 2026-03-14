# Homebrew formula template for repolyze (manual fallback).
# Normally cargo-dist generates and publishes this to maximgorbatyuk/homebrew-tap.
# Use this template only if the automated publish fails.
class Repolyze < Formula
  desc "Repository analytics for local Git repositories"
  homepage "https://github.com/maximgorbatyuk/repolyze"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/maximgorbatyuk/repolyze/releases/download/v#{version}/repolyze-cli-aarch64-apple-darwin.tar.xz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end

    on_intel do
      url "https://github.com/maximgorbatyuk/repolyze/releases/download/v#{version}/repolyze-cli-x86_64-apple-darwin.tar.xz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  def install
    bin.install "repolyze"
  end

  test do
    assert_match "repolyze", shell_output("#{bin}/repolyze --help")
  end
end
