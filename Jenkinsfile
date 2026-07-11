// CI/CD for gbif/name-parser-rust — the Java FFM binding (bindings/java), deployed to GBIF Nexus.
// Modelled on the CatalogueOfLife/backend Jenkinsfile (same shared library, tools, Maven settings
// config, and release flow), with one addition: the Java JAR bundles the nameparser-ffi Rust
// cdylib, so we `cargo build` it before the Maven build. The Rust engine + the Python/R/CLI
// artifacts publish to their own channels (crates.io / PyPI / CRAN / GitHub Releases), not here —
// see DISTRIBUTION.md.
//
// NOTE: the agent must have a Rust toolchain (rustup / ~/.cargo) for the cdylib step. The single
// agent builds the cdylib for ITS platform (GBIF's build agents are Linux → linux-x86_64), which
// is the deploy target; a multi-OS fat JAR would need a matrix or cross-compile (DISTRIBUTION.md §3).

@Library('gbif-common-jenkins-pipelines') _

pipeline {
  agent any
  tools {
    maven 'Maven 3.9.9'
    jdk 'LibericaJDK25'
  }
  options {
    disableConcurrentBuilds()
    buildDiscarder(logRotator(numToKeepStr: '10'))
    skipDefaultCheckout(true)   // disables auto checkout - we wipe the workspace here
    skipStagesAfterUnstable()
    timestamps()
  }
  parameters {
    separator(name: "release_separator", sectionHeader: "Release Parameters")
    booleanParam(name: 'RELEASE', defaultValue: false, description: 'Do a Maven release of the Java FFM binding (bindings/java)')
    string(name: 'RELEASE_VERSION', defaultValue: '', description: 'Release version (optional)')
    string(name: 'DEVELOPMENT_VERSION', defaultValue: '', description: 'Development version (optional)')
  }
  stages {
    stage('Checkout') {
      steps {
        deleteDir()             // clean workspace
        checkout scm            // fresh clone
      }
    }

    // The Java FFM binding bundles the nameparser-ffi cdylib into its JAR (bindings/java/pom.xml
    // copies target/release/libnameparser_ffi.* into native/${os.detected.classifier}/), so build
    // the release cdylib first. The script bootstraps rustup if the agent has no Rust, so this
    // runs on a plain `agent any`.
    stage('Build native cdylib') {
      steps {
        sh 'bash ci/build-cdylib.sh'
      }
    }

    stage('Maven build') {
      when {
        allOf {
          not { expression { params.RELEASE } };
        }
      }
      steps {
        withMaven(
          globalMavenSettingsConfig: 'org.jenkinsci.plugins.configfiles.maven.GlobalMavenSettingsConfig1387378707709',
          mavenOpts: '-Xmx2048m -Dorg.slf4j.simpleLogger.showDateTime=true -Dorg.slf4j.simpleLogger.dateTimeFormat=HH:mm:ss,SSS',
          mavenSettingsConfig: 'b043019e-79d8-48fd-8ecf-b20e3fb0a3cc',
          traceability: true
        ) {
          sh '''mvn -f bindings/java/pom.xml clean -U deploy'''
        }
      }
    }

    // NOTE: this release stage mirrors CoL's structure but needs two things before it works for a
    // single native-binding module: (1) an <scm> block in bindings/java/pom.xml (release:prepare
    // tags via SCM); (2) the cdylib inside release:perform's fresh target/checkout build — the
    // build-cdylib.sh below builds it in the OUTER workspace, not that inner checkout. The snapshot
    // deploy ('Maven build' above) is the working every-commit flow; finish release wiring later.
    stage('Maven release: Java FFM binding') {
      when {
        allOf {
          expression { params.RELEASE };
          branch 'master';
        }
      }
      steps {
        script {
          def releaseArgs = utils.createReleaseArgs(params.RELEASE_VERSION, params.DEVELOPMENT_VERSION, false)
          configFileProvider(
            [configFile(fileId: 'org.jenkinsci.plugins.configfiles.maven.GlobalMavenSettingsConfig1387378707709',
              variable: 'MAVEN_SETTINGS_XML')]) {
            git 'https://github.com/gbif/name-parser-rust.git'
            sh 'bash ci/build-cdylib.sh'
            sh "mvn -s \$MAVEN_SETTINGS_XML -f bindings/java/pom.xml -B -Denforcer.skip=true -Darguments=\"-DskipTests -DskipITs\" release:prepare release:perform -Dtag=v${params.RELEASE_VERSION} ${releaseArgs}"
          }
        }
      }
    }
  }

  post {
    success {
      echo 'Pipeline executed successfully!'
    }
    failure {
      echo 'Pipeline execution failed!'
    }
  }
}
