use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        pi: {
            all(
                target_arch = "aarch64",
                target_os = "linux",
                target_env = "gnu"
            )
        }
    }
}
