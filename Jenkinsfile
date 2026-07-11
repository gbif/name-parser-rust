// Jenkinsfile — CI/CD for gbif/name-parser-rust
//
// Implements the pipeline described in DISTRIBUTION.md §4: build the nameparser-ffi cdylib
// per platform, then package/publish each binding to its own channel. This is a STARTING
// POINT — every value marked `ADJUST` must be matched to your Jenkins: agent labels, the
// JDK-22 tool name, and credential / managed-file IDs. Nothing here hardcodes a secret.
//
// Recommended first milestone (DISTRIBUTION.md §4): run with NATIVE_PLATFORMS=linux-x86_64
// and DEPLOY_JAVA=true to get the Java FFM artifact onto repository.gbif.org; enable the
// other platforms/channels as agents and tokens become available.

// ---- Platform metadata. Host-native builds (one agent per target). Extend with
//      cross-compilation (cargo-zigbuild) if you'd rather build on fewer nodes.
//      `res` MUST match the classifier Ffi.java computes when it extracts the bundled
//      library at runtime (DISTRIBUTION.md §3a). ----
def NATIVE = [
  'linux-x86_64'  : [ lib: 'libnameparser_ffi.so',    res: 'linux-x86_64'   ],
  'linux-aarch64' : [ lib: 'libnameparser_ffi.so',    res: 'linux-aarch_64' ],
  'darwin-arm64'  : [ lib: 'libnameparser_ffi.dylib', res: 'osx-aarch_64'   ],
  'darwin-x86_64' : [ lib: 'libnameparser_ffi.dylib', res: 'osx-x86_64'     ],
  'windows-x86_64': [ lib: 'nameparser_ffi.dll',      res: 'windows-x86_64' ],
]

pipeline {
  agent none

  options {
    timestamps()
    disableConcurrentBuilds()
    buildDiscarder(logRotator(numToKeepStr: '20'))
    timeout(time: 90, unit: 'MINUTES')
  }

  parameters {
    string(name: 'NATIVE_PLATFORMS', defaultValue: 'linux-x86_64',
           description: 'Comma-separated cdylib targets to build (keys of the NATIVE map). Add a platform only where a matching agent exists.')
    booleanParam(name: 'DEPLOY_JAVA', defaultValue: false,
           description: 'mvn deploy the Java FFM binding to repository.gbif.org (else just verify).')
    booleanParam(name: 'PUBLISH_PYTHON', defaultValue: false, description: 'Build + upload Python wheels to PyPI.')
    booleanParam(name: 'PUBLISH_R',      defaultValue: false, description: 'Build + check the R package; archive the tarball.')
    booleanParam(name: 'PUBLISH_CLI',    defaultValue: false, description: 'Build + archive native CLI binaries.')
  }

  stages {

    // Fast PR gate: the pure-Rust crates build, test, and lint clean. nameparser-py is
    // excluded (it needs a Python/pyo3 toolchain and is validated in the Python stage); the
    // clippy gate is scoped to the crates verified `-D warnings`-clean (nameparser-py emits
    // known pyo3-0.22 macro false positives — see the chore commit in git history).
    stage('Core: build, test, lint') {
      agent { label 'linux && amd64' }   // ADJUST
      steps {
        sh '''
          set -eu
          . "$HOME/.cargo/env"
          cargo --version && rustc --version
          cargo test --workspace --exclude nameparser-py
          cargo clippy -p nameparser -p nameparser-cli --all-targets -- -D warnings
        '''
      }
    }

    // One cdylib per requested platform, each on its own agent, stashed for the Java stage.
    stage('Native cdylib matrix') {
      matrix {
        axes {
          axis {
            name 'PLATFORM'
            values 'linux-x86_64', 'linux-aarch64', 'darwin-arm64', 'darwin-x86_64', 'windows-x86_64'
          }
        }
        stages {
          stage('cdylib') {
            when { expression { params.NATIVE_PLATFORMS.split(',').collect { it.trim() }.contains(env.PLATFORM) } }
            // ADJUST: map each PLATFORM to a real node label. Convention assumed here:
            // a node labelled with both `rust` and the platform key.
            agent { label "rust && ${PLATFORM}" }
            steps {
              script {
                def meta = NATIVE[env.PLATFORM]
                if (isUnix()) {
                  sh """
                    set -eu
                    . "\$HOME/.cargo/env"
                    cargo build --release -p nameparser-ffi
                    test -f target/release/${meta.lib}
                  """
                } else {
                  // Windows agent: cargo must already be on PATH (rustup default install).
                  bat """
                    cargo build --release -p nameparser-ffi
                    if not exist target\\release\\${meta.lib} exit 1
                  """
                }
                stash name: "cdylib-${env.PLATFORM}", includes: "target/release/${meta.lib}"
              }
            }
          }
        }
      }
    }

    // Lay the built cdylibs into the classpath resource tree, run the FFM tests against the
    // linux one, and (optionally) deploy. NOTE: packaging the resources makes them available
    // in the JAR, but runtime loading for downstream consumers additionally needs the
    // Ffi.java bundle-and-extract change (DISTRIBUTION.md §3a / §6); until then Ffi still
    // resolves via -Dnameparser.ffi.lib, which is exactly how the test run below works.
    stage('Java FFM: package + deploy') {
      agent { label 'linux && amd64' }   // ADJUST
      tools { jdk 'jdk-22' }             // ADJUST: a configured JDK 22+ (FFM is finalized in 22)
      steps {
        script {
          def platforms = params.NATIVE_PLATFORMS.split(',').collect { it.trim() }.findAll { it }
          for (p in platforms) {
            def meta = NATIVE[p]
            unstash "cdylib-${p}"   // restores target/release/<lib>
            sh """
              set -eu
              mkdir -p bindings/java/src/main/resources/native/${meta.res}
              mv target/release/${meta.lib} bindings/java/src/main/resources/native/${meta.res}/${meta.lib}
            """
          }
          def goal = params.DEPLOY_JAVA ? 'deploy' : 'verify'
          def linuxLib = "${env.WORKSPACE}/bindings/java/src/main/resources/native/linux-x86-64/libnameparser_ffi.so"
          // ADJUST: managed Maven settings.xml with the repository.gbif.org server creds.
          // (Alternatively use withMaven { } if that's your GBIF convention.)
          configFileProvider([configFile(fileId: 'gbif-maven-settings', variable: 'MAVEN_SETTINGS')]) {
            sh """
              set -eu
              mvn -B -s "\$MAVEN_SETTINGS" -f bindings/java/pom.xml clean ${goal} \\
                  -Dnameparser.ffi.lib='${linuxLib}'
            """
          }
        }
      }
      post {
        always {
          junit allowEmptyResults: true, testResults: 'bindings/java/target/surefire-reports/*.xml'
        }
      }
    }

    // ---- Optional publish channels (off by default; each targets a non-Maven registry). ----

    stage('Python wheels → PyPI') {
      when { expression { params.PUBLISH_PYTHON } }
      agent { label 'linux && amd64' }   // ADJUST: cibuildwheel drives Docker for manylinux
      steps {
        sh '''
          set -eu
          . "$HOME/.cargo/env"
          python3 -m pip install --upgrade cibuildwheel twine
          python3 -m cibuildwheel --output-dir wheelhouse crates/nameparser-py
        '''
        // ADJUST: a PyPI API token credential (Secret text).
        withCredentials([string(credentialsId: 'pypi-token', variable: 'TWINE_PASSWORD')]) {
          sh 'TWINE_USERNAME=__token__ python3 -m twine upload wheelhouse/*.whl'
        }
      }
      post { always { archiveArtifacts artifacts: 'wheelhouse/*.whl', allowEmptyArchive: true } }
    }

    stage('R package') {
      when { expression { params.PUBLISH_R } }
      agent { label 'linux && amd64' }   // ADJUST: needs R (with rextendr) + a Rust toolchain
      steps {
        sh '''
          set -eu
          . "$HOME/.cargo/env"
          Rscript -e 'rextendr::document("bindings/r")'
          R CMD build bindings/r
          R CMD check --no-manual nameparser_*.tar.gz
        '''
      }
      post { always { archiveArtifacts artifacts: 'nameparser_*.tar.gz', allowEmptyArchive: true } }
    }

    stage('CLI binaries') {
      when { expression { params.PUBLISH_CLI } }
      agent { label 'linux && amd64' }   // ADJUST: or reuse the cdylib matrix for multi-platform
      steps {
        sh '''
          set -eu
          . "$HOME/.cargo/env"
          cargo build --release -p nameparser-cli
          tar -C target/release -czf nameparser-cli-linux-x86_64.tar.gz nameparser-cli
        '''
      }
      post { always { archiveArtifacts artifacts: 'nameparser-cli-*.tar.gz', allowEmptyArchive: true } }
    }
  }

  post {
    success { echo "OK — native platforms: ${params.NATIVE_PLATFORMS}; java deploy: ${params.DEPLOY_JAVA}" }
    failure { echo 'Pipeline FAILED' }
  }
}
