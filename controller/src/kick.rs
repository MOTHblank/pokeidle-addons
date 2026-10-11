// KICK stream data reported by the Controller Bridge.
// Browser/session management was removed; the controller now displays MultiKick links
// from detected live channel URLs rather than managing a separate KICK browser.
#[derive(Clone, Debug)]
pub struct KickStream {
    pub url: String,
}
