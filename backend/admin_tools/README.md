# admin_tools

## A Docker image that contains a suite of compilers and tools for devopsy/dev things. It serves 2 purposes
1. As a remote container for development. It runs on a dev server and monitors changes that are pushed to the host
   over a tool like mutagen and recompiles and / or rebuilds , reloads code. It's often useful to build on the actual architecture / OS 
   particularly when developing on Mac.  
   
  - imagine developing locally with local tools, having the project auto sync to a dev server where processes are listening 
  - to rebuild / recompile and reload the project. 
  - an existing installation of a set of containers is running in "dev mode" which simply maps a volume to the build output of a project
  - thereby shadowing the containers build it binary or code files. This allows us to use prod status containers with dev code. we don't need to 
  - change port numbers, run competing containers or special dev version containers. etc. We just temporalily swap builds. 
    
2. A production server admin toolkit
   - keeps the host clean and minimal. All tooling runs in the container in distrobox. 
   - distrobox gives us easy access the host which is important for admin / troubleshooting work, feels like working on the host
   - allows us to download and update packages without polluting the prod server. 
   - we could put prod containers into dev mode and iterate over quick fixes, bug fixes and give us time to properly push an update.
 
## Example

we install tools like Mise, zellij, tmux, fd, ripgrep, neovim, bacon,  and whatever else that makes sense.
