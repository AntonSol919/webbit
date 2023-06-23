# Some notes on using/running webbit

quarantine and webbit should have two different subdomains.
Edit Rocket.toml to fit your host.

./vouch is a script that is run for each upload. If it returns 0 the packet is accepted.

Peering can be as simple as `lk watch-log webbit:[#:pub] -- hop:=:[u32:0] :follow | nc peer_ip -p 5030` and doing `nc -l 5030 | lk save`
You can do far more interesting peering strategies with `lk route` that i don't have time to expand on right now.
