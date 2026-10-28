# Homebrew formula template for Zacess Scenes. Replace VERSION with the release version (without
# the v) and each SHA256_ placeholder with the sum from the matching .sha256 file on the release,
# then copy it to Formula/zacess-scenes.rb in the zacesshq/homebrew-tap repository.
class ZacessScenes < Formula
  desc "Calm animated scenes for the terminal in the Zacess high contrast themes"
  homepage "https://zacess.com"
  version "VERSION"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/zacesshq/scenes/releases/download/vVERSION/zacess-scenes-vVERSION-aarch64-apple-darwin.tar.gz"
      sha256 "SHA256_MACOS_ARM"
    end
    on_intel do
      url "https://github.com/zacesshq/scenes/releases/download/vVERSION/zacess-scenes-vVERSION-x86_64-apple-darwin.tar.gz"
      sha256 "SHA256_MACOS_INTEL"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/zacesshq/scenes/releases/download/vVERSION/zacess-scenes-vVERSION-aarch64-unknown-linux-musl.tar.gz"
      sha256 "SHA256_LINUX_ARM"
    end
    on_intel do
      url "https://github.com/zacesshq/scenes/releases/download/vVERSION/zacess-scenes-vVERSION-x86_64-unknown-linux-musl.tar.gz"
      sha256 "SHA256_LINUX_INTEL"
    end
  end

  def install
    bin.install "zacess-scenes"
  end

  test do
    assert_match "zacess-scenes", shell_output("#{bin}/zacess-scenes --version")
    system bin/"zacess-scenes", "sea", "--headless", "--frames", "1", "--size", "20x8", "--theme", "deep-dark"
  end
end
