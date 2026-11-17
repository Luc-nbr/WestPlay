// Euro-Hub, in Discord's sign-in window (signin.rs): Discord's own login page. Discord is a single page app (its
// address changes without a page loading), so this watches where it is: once in the app (its channels), it says the
// page is signed in, nothing more of the account.
;(() => {
  if (location.hostname !== 'discord.com' || window.top !== window) return
  const post = (message) => window.chrome?.webview?.postMessage(message)
  const look = () => {
    if (location.pathname.startsWith('/channels') || location.pathname === '/app') {
      post({ signedIn: true })
      return
    }
    setTimeout(look, 500)
  }
  look()
})()
