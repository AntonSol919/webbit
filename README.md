⚠️This is very much a prototype.⚠️

# Webbit - Crowdsourcing<sup>2</sup>

As you've probably heard, `<Social App>` is about to `<shit the bed | abuse their position>`.
Anyone whose been around for a while can see the similarities with `<other fuckup>`,
and who can forget that big controversy in the year `<random.randrange(2004,2025)>`

## What is Webbit?

- ~A quick and dirty hack to get a markup language (html) into [linkspace](https://www.linkspace.dev)~  
- ~A new kind of platform.~  
- ~The ultimate code gulf challange~  
- ~A dumpster fire waiting to happen~  
- ~A self-improving POC?~
- ~I don't know.~
- Whatever you make of it
    
Open a page, edit it, save it.

> What page?

Goto any webbit page ending in .html
For example [http://webbit.alinkspace.org/some_page.html](http://webbit.alinkspace.org/some_page.html)

(Ok this one URL is a bit special because its pinned and wont update, but any other path_name.html url can be edited!)

> The editor looks like trash?

Thats not a question?

But luckily you don't have to use that editor. 
Any URL starting with webbit. and ending in .html can be edited.
Just add ?uploader to the URL and press F12  and edit it using the browser tools (or check the box and start typing).

> That's not a very user friendly editor

Well, instead of creating a page you can also update the editor to make it user friendly.

> Wait you're letting every one upload custom javascript with their post?

Yes!

> Can I create a 2000-esque post with music and blinking text?:

Yes its great!
Or ~steal~ borrow something a little less offensive from another page.

> Cool, but what about comments. Every good website has comments.

Isn't it obvious? 

Create a page -> to create a page -> to create comments. [example.forum.editor.html](./static/example.forum.editor.html)

> So you made javascript injections attacks a feature?

Yes!

## Saving a page.

Once you created a page you will be directed towards 'quarantine'.
Here you vouch for it with a digital signature.
Depending on the host, they'll accept your key and that's it.
The final result is uploaded and accessible at the path you chose.

You'll be redirected there with a path including ?hash=... . this is the permanent link.
Requesting a page without ?hash=... gives you the latest upload. 

## That's cool but what about the issue with `[Social App]`

I promise you can really trust me and i'll not abuse my administrative power.
This time will be different. Pinky promise.
I'm really in it to build a community, and you can trust this mission will never change.

Just kidding, that would be insanity.

The root issue with `[Social App]` is:

- you're talking _at_ a server.
- and you explicitly agreed that the host has final and total administrative control
- and the host inevitably wants to leverage that position.

Webbit is a [linkspace](www.linkspace.dev) application. Linkspace is a supernet (Think 'like Git' but for everything but code).
If for whatever reason you think i'm a trash host, you just have to find a different way to talk _about_ the data.

Because at the end of the day, data is fungible and you only need a way to reuse and reshare.
