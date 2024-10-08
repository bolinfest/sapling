# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"

setup configuration

  $ setup_common_config blob_files
  $ cd "$TESTTMP/mononoke-config"
  $ cat >> repos/repo/server.toml << EOF
  > [[bookmarks]]
  > name="main"
  > [[bookmarks.hooks]]
  > hook_name="limit_filesize"
  > [[bookmarks]]
  > regex=".*"
  > hooks_skip_ancestors_of=["main"]
  > [[bookmarks.hooks]]
  > hook_name="limit_filesize"
  > [[bookmarks]]
  > regex="^tag/.*"
  > allow_move_to_public_commits_without_hooks=true
  > [[bookmarks.hooks]]
  > hook_name="limit_filesize"
  > [[hooks]]
  > name="limit_filesize"
  > config_string_lists={filesize_limits_regexes=[".*"]}
  > config_int_lists={filesize_limits_values=[10]}
  > bypass_pushvar="ALLOW_LARGE_FILES=true"
  > EOF

  $ merge_just_knobs <<EOF
  > {
  >   "bools": {
  >     "scm/mononoke:run_hooks_on_additional_changesets": true
  >   }
  > }
  > EOF
  $ setup_common_hg_configs
  $ cd $TESTTMP

setup repo
  $ start_and_wait_for_mononoke_server
  $ cd $TESTTMP
  $ hg clone -q mono:repo repo
  $ cd repo
  $ echo B > B
  $ hg add B
  $ hg ci -m 'B'
  $ hg push -r . --to main --create
  pushing rev c0e1f5917744 to destination mono:repo bookmark main
  searching for changes
  exporting bookmark main

Try to pushrebase new commit that fails the hook - push should fail
  $ echo "aaaaaaaaaaaaaa" > large_file
  $ hg add large_file
  $ hg ci -m 'large_commit'
  $ hg push -r . --to tag/newtag --create
  pushing rev cd415129827a to destination mono:repo bookmark tag/newtag
  searching for changes
  remote: Command failed
  remote:   Error:
  remote:     hooks failed:
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote: 
  remote:   Root cause:
  remote:     hooks failed:
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote: 
  remote:   Debug context:
  remote:     "hooks failed:\nlimit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: \".*\". See https://fburl.com/landing_big_diffs for instructions.\nlimit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: \".*\". See https://fburl.com/landing_big_diffs for instructions."
  abort: unexpected EOL, expected netstring digit
  [255]

We are not uploading a new commit, since it's already in commit cloud, but
the push should fail nevertheless. But first let's check that commit actually exists on
mononoke now
  $ mononoke_admin phases fetch cd415129827add17f8486647dad5f3f84f5df316
  * using repo "repo" repoid RepositoryId(0) (glob)
  draft
  $ hg push -r . --to tag/newtag --create
  pushing rev cd415129827a to destination mono:repo bookmark tag/newtag
  searching for changes
  remote: Command failed
  remote:   Error:
  remote:     hooks failed:
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote: 
  remote:   Root cause:
  remote:     hooks failed:
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote: 
  remote:   Debug context:
  remote:     "hooks failed:\nlimit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: \".*\". See https://fburl.com/landing_big_diffs for instructions.\nlimit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: \".*\". See https://fburl.com/landing_big_diffs for instructions."
  abort: unexpected EOL, expected netstring digit
  [255]

Now let's move another bookmark to this commit to make it public
First check that push fail for this bookmark as well
  $ hg push -r . --to another_bookmark --create
  pushing rev cd415129827a to destination mono:repo bookmark another_bookmark
  searching for changes
  remote: Command failed
  remote:   Error:
  remote:     hooks failed:
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote: 
  remote:   Root cause:
  remote:     hooks failed:
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote: 
  remote:   Debug context:
  remote:     "hooks failed:\nlimit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: \".*\". See https://fburl.com/landing_big_diffs for instructions."
  abort: unexpected EOL, expected netstring digit
  [255]
  $ hg push -r . --to another_bookmark --create --pushvar ALLOW_LARGE_FILES=true
  pushing rev cd415129827a to destination mono:repo bookmark another_bookmark
  searching for changes
  exporting bookmark another_bookmark

Try the push tag/newtag again. Since this commit is public it should succeed
  $ mononoke_admin phases fetch cd415129827add17f8486647dad5f3f84f5df316
  * using repo "repo" repoid RepositoryId(0) (glob)
  public
  $ hg push -r . --to tag/newtag --create
  pushing rev cd415129827a to destination mono:repo bookmark tag/newtag
  searching for changes
  no changes found
  exporting bookmark tag/newtag

Try to push another bookmark that doesn't match the regex. This bookmark should fail
  $ hg push -r . --to notag/newtag --create
  pushing rev cd415129827a to destination mono:repo bookmark notag/newtag
  searching for changes
  no changes found
  remote: Command failed
  remote:   Error:
  remote:     hooks failed:
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote: 
  remote:   Root cause:
  remote:     hooks failed:
  remote:     limit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: ".*". See https://fburl.com/landing_big_diffs for instructions.
  remote: 
  remote:   Debug context:
  remote:     "hooks failed:\nlimit_filesize for cd415129827add17f8486647dad5f3f84f5df316: File size limit is 10 bytes. You tried to push file large_file that is over the limit (15 bytes). This limit is enforced for files matching the following regex: \".*\". See https://fburl.com/landing_big_diffs for instructions."
  abort: unexpected EOL, expected netstring digit
  [255]
