# IPTorrents sign-in fixtures

These pages are **written by hand** from the structure that Jackett's and Prowlarr's IPTorrents
definitions and public bug reports describe (a `login.php` form posting to `take_login.php`, with
`username` and `password` fields; session cookies `uid` and `pass`; a CAPTCHA, a Cloudflare challenge
or two-factor step on some accounts). They are **not recordings of the real site**: no request was
made to it. They exist so the form handling and cookie capture have something concrete to run
against; the real flow has to be verified by someone with an account.
