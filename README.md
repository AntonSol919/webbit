⚠️This is very much a prototype.⚠️

# Webbit - Crowdsourcing<sup>2</sup>

As you've probably heard, `<Social App>` is about to `<shit the bed | abuse their position>`.
Anyone whose been around for a while can see the similarities with `<other fuckup>`,
and who can forget that big controversy in the year `<random.randrange(2004,2025)>`

## What is Webbit?

- ~A quick and dirty hack to bridge [linkspace](https://www.linkspace.dev)<->http to visualize it with html~  
- ~A new kind of platform.~  
- ~The ultimate code golf challange~  
- ~A dumpster fire waiting to happen~  
- ~A self-improving POC~  
- ~I don't know.~  
- Whatever you make of it
    
Open a page, edit it, save it.

> What page?

Goto any server running webbit page and open a page ending in .html
For example [http://webbit.alinkspace.org/some_page.html](http://webbit.alinkspace.org/some_page.html)

(Ok this one URL is a bit special because its version is pinned, but editing any other path_name.html makes the latest edit available)

> The editor looks like trash?

Thats not a question?

But luckily you don't have to use that editor.
Any URL starting with webbit. and ending in .html can be edited.
Just add ?uploader to the URL and press F12  and edit it using the browser tools (or use the 'Edit' checkbox)

> That's not a very user friendly editor

Well, instead of creating a page you can also update the editor to make it user friendly.

> Wait... You're letting every one upload custom javascript with their page?

Yes!

> Can I create a 2000-esque post with music and blinking text?:

Yes its great!
Or ~steal~ borrow something a little less offensive from another page.

> Cool, but what about comments. Every good website has comments.

Isn't it obvious? 

Create a page -> to create a page -> to create comments. [example.forum.editor.html](./static/example.forum.editor.html)

> What about feature X 

Just build and share it yourself.

> So you made javascript injections attacks a feature?

Yes!

> Isn't that dangerous? 

No - just PEBCAK.

There are four 'dangers' when allowing users to edit a website.

- The 'Fake login' danger: Someone uploads a page that looks like a bank account or facebook.
This is not specific to Webbit. All browser security requires users to check the Domain/URL.

- The 'Steal your Webbit account' danger: Someone uploads javascript to steal your account and cookies.
JavaScript can't access any data outside its subdomain (i.e. webbit.alinkspace.org)[^1].
Furthermore, webbit doesn't use cookies or accounts. Instead you generate a linkspace identity and create a cryptographic signature of the page on the client side.

[^1]: Except when a domain has specifically configured information to be accessible from the outside.

- The 'Mine for crypto' danger: Someone uploads a crypto miner to make money by using your electricity.
This is not specific to Webbit. And the process stops the moment you leave the page.

- The 'upload illegal stuff' danger: Somebody uploads nasty stuff
Did you know you can encrypt and encode any data into video or text and YouTube or Facebook could do nothing about it? 

On the other side - its data that is readable and you don't want it - the [./vouch](./vouch) script can be updated to check whitelists, blacklists, 
or any other filter you can imagine.

> Whats the difference with FTP, WebDAV, or other [alternative](https://en.wikipedia.org/wiki/WebDAV#Alternatives_to_WebDAV)?

They address a server.
Webbit writes to linkspace and shares it with one or more servers.

Furthermore, AFAIK they have no in-browser editing.

## Saving a page.

Once you created a page you will be directed towards 'quarantine'.
Here you vouch for your contribution with a digital signature.
Depending on the host, they'll accept your key and that's it.
The final result is uploaded and accessible at the path you chose.

## Overwriting a page

The latest upload is served by default.  
To get a older version use ?hash=...  

## That's cool but what about the issue with `[Social App]`

I promise you can really trust me and i'll not abuse my administrative power.
This time will be different. Pinky promise.
I'm really in it to build a community, and you can trust this mission will never change.

Just kidding - going to the next `[Social App]` and wait for the next `<bad thing>` would be insanity.

The root issue with `[Social App]` is:

- you're talking _at_ a server.
- and you explicitly agreed that the host has final and total administrative control
- and the host inevitably wants to leverage that position.

Webbit is a [linkspace](https://www.linkspace.dev) application. Linkspace is a supernet (Think 'like Git' but for everything but code).
An admin can be ignored, a host can be dropped, without ever losing the history of events.
It is a protocol about to talk _about_ data.
