# remotenames.py
#
# Copyright 2017 Augie Fackler <raf@durin42.com>
# Copyright 2017 Sean Farley <sean@farley.io>
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2 or any later version.

from __future__ import absolute_import

from .node import hex

from . import (
    vfs as vfsmod,
)

# directory name in .hg/ in which remotenames files will be present
remotenamedir = 'remotenames'

def writeremotenamefile(repo, remotepath, names, nametype):
    vfs = vfsmod.vfs(repo.vfs.join(remotenamedir))
    f = vfs(nametype, 'w', atomictemp=True)
    # write the storage version info on top of file
    # version '0' represents the very initial version of the storage format
    f.write('0\n\n')

    for name, node in sorted(names.iteritems()):
        if nametype == "branches":
            for n in node:
                f.write('%s\0%s\0%s\n' % (n, remotepath, name))
        elif nametype == "bookmarks":
            if node:
                f.write('%s\0%s\0%s\n' % (node, remotepath, name))

    f.close()

def saveremotenames(repo, remotepath, branches=None, bookmarks=None):
    """
    save remotenames i.e. remotebookmarks and remotebranches in their
    respective files under ".hg/remotenames/" directory.
    """
    wlock = repo.wlock()
    try:
        if bookmarks:
            writeremotenamefile(repo, remotepath, bookmarks, 'bookmarks')
        if branches:
            writeremotenamefile(repo, remotepath, branches, 'branches')
    finally:
        wlock.release()

def pullremotenames(localrepo, remoterepo):
    """
    pulls bookmarks and branches information of the remote repo during a
    pull or clone operation.
    localrepo is our local repository
    remoterepo is the peer instance
    """
    remotepath = remoterepo.url()
    bookmarks = remoterepo.listkeys('bookmarks')
    # on a push, we don't want to keep obsolete heads since
    # they won't show up as heads on the next pull, so we
    # remove them here otherwise we would require the user
    # to issue a pull to refresh the storage
    bmap = {}
    repo = localrepo.unfiltered()
    for branch, nodes in remoterepo.branchmap().iteritems():
        bmap[branch] = []
        for node in nodes:
            if node in repo and not repo[node].obsolete():
                bmap[branch].append(hex(node))

    saveremotenames(localrepo, remotepath, bmap, bookmarks)
