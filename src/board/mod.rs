//! ボード固有コードのエントリポイントです。

// ボードごとのモジュールを条件コンパイルで切り替えます。
// `--features rp2040` を指定してビルドすることで rp2040 の設定が使われます。
#[cfg(feature = "rp2040")]
pub mod rp2040;

#[cfg(feature = "rp2040")]
pub use rp2040::*;

// RP2350 board module
#[cfg(feature = "rp2350")]
pub mod rp2350;

#[cfg(feature = "rp2350")]
pub use rp2350::*;

// BSP の再エクスポートモジュールを公開します（src/board/bsp.rs を参照）
pub mod bsp;

// `bsp` モジュールが持つ主要な再エクスポートを board ルートでも公開します。
// これにより既存コードの `use board::hal` / `use board::entry` が動作します。
pub use bsp::{entry, hal};
