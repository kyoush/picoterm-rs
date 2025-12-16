//! BSP（ボードサポートパッケージ）の選択を抽象化するモジュール。
//!
//! feature フラグによって適切な BSP クレートを re-export します。
//! まずは `rp2040`（rp-pico）をサポートします。

// RP2040 用 BSP を再エクスポート（モジュールのルートに rp_pico の項目を公開）
#[cfg(feature = "rp2040")]
pub use rp_pico::entry;

#[cfg(feature = "rp2040")]
pub use rp_pico::hal;

// 将来的に RP2350 をサポートする場合はここに条件付き re-export を追加してください。
#[cfg(feature = "rp2350")]
pub use rp235x_hal::entry;

// Re-export the HAL crate as `hal` so code that expects `board::hal` works
// the same way as with the rp2040 BSP (`rp_pico::hal`).
#[cfg(feature = "rp2350")]
pub use rp235x_hal as hal;

// 安全のため、どのボード feature も指定されていない場合は明示的にエラーにします。
#[cfg(not(any(feature = "rp2040", feature = "rp2350")))]
compile_error!(
    "No board feature selected. Enable `--features rp2040` or implement other board support."
);
