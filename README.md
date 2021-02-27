# dlnaproxy

`dlnaproxy` is meant to enable the use of a DLNA server (ex: MiniDLNA) past the local network boundary.

## Use case
Let's say you're hosting a media library on a remote server. It might be because that remote server has more bandwith, more storage, or both.\
It can also be your self-hosted NAS that you are trying to access from a remote location.
If you're able to connect to that server, either through a VPN or because the machine is routed directly on the Internet, `dlnaproxy` will attempt to connect to that server and if successful, it will announce it on your current LAN as if that server were there.


## The diagram
```
          Network boundary                 +------------------+
                ++          connect back   |                  |
     +----------++-------------------------+       you        |
     |          ||                         |                  |
     |          ||                         +---^--------------+
+----v-----+    ||   +------------+            |
| Remote   |    ||   |            +------------+
| DLNA     <----++---+ dlnaproxy  |    broadcast
| Server   | fetch info           |
|          |    ++   |            |
+----------+    ||   +------------+
                ||
                ||
                ++
```
